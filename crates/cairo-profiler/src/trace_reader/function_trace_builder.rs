use anyhow::{Context, Result};
use cairo_lang_sierra::extensions::core::{CoreConcreteLibfunc, CoreLibfunc, CoreType};
use cairo_lang_sierra::program::{GenStatement, Program, ProgramArtifact, StatementIdx};
use cairo_lang_sierra::program_registry::ProgramRegistry;
use cairo_lang_sierra_to_casm::compiler::CairoProgramDebugInfo;
use itertools::{chain, Itertools};
use regex::Regex;
use std::collections::HashMap;
use trace_data::TraceEntry;

const MAX_TRACE_DEPTH: u8 = 100;

/// Index according to the list of functions in the sierra program.
type UserFunctionIndex = usize;
type WeightInSteps = usize;
type FunctionName = String;

enum MaybeSierraStatementIndexFromPc {
    SierraStatementIndex(StatementIdx),
    PcOutOfFunctionArea,
}

/// Collects profiling info of the current run using the trace.
// TODO: fix contracts
// TODO 2: inlined functions
// TODO 3: connecting appropriate function with entrypoint call
pub fn collect_profiling_info(
    trace: &[TraceEntry],
    program_artifact: &ProgramArtifact,
    casm_debug_info: &CairoProgramDebugInfo,
    was_run_with_header: bool,
) -> Result<(Vec<(Vec<FunctionName>, WeightInSteps)>, WeightInSteps)> {
    let program = &program_artifact.program;
    let sierra_program_registry = &ProgramRegistry::<CoreType, CoreLibfunc>::new(program).unwrap();

    // Some CASM programs starts with a header of instructions to wrap the real program.
    // `real_pc_0` is the PC in the trace that points to the same CASM instruction which is in
    // the real PC=0 in the original CASM program. That is, all trace's PCs need to be
    // subtracted by `real_pc_0` to get the real PC they point to in the original CASM
    // program.
    // This is the same as the PC of the last trace entry plus 1, as the header is built to have
    // a `ret` last instruction, which must be the last in the trace of any execution.
    // The first instruction after that is the first instruction in the original CASM program.
    // This logic only applies when a header was added to the CASM program.
    let real_pc_0 = if was_run_with_header {
        trace.last().unwrap().pc + 1
    } else {
        1
    };

    // The function stack trace of the current function, excluding the entrypoint function.
    // Represented as a vector of indices of the functions
    // in the stack (indices of the functions according to the list in the sierra program).
    // Limited to depth `max_stack_trace_depth`.
    let mut function_stack: Vec<(UserFunctionIndex, WeightInSteps)> = vec![];
    // Tracks the depth of the function stack, without limit. This is usually equal to
    // `function_stack.len()`, but if the actual stack is deeper than `max_stack_trace_depth`,
    // this remains reliable while `function_stack` does not.
    let mut function_stack_depth: usize = 0;
    // Weight of the current function in steps.
    let mut current_function_weight = 0;
    // The key is a function stack trace (see `function_stack`, but including the current
    // function).
    // The value is the weight of the stack trace so far, not including the pending weight being
    // tracked at the time.
    let mut stack_trace_weights = HashMap::new();

    let mut end_of_program_reached = false;

    // Those are counter separately and then displayed as steps of the entrypoint in the profile
    // tree since technically they don't belong to any function, but still increase the number of
    // total steps. The value is different from zero only for functions run with header.
    let mut header_and_footer_steps = 0;

    for step in trace {
        // Skip the header. This only makes sense when a header was added to CASM program.
        if step.pc < real_pc_0 && was_run_with_header {
            header_and_footer_steps += 1;
            continue;
        }
        let real_pc = step.pc - real_pc_0;
        // Skip the footer. This only makes sense when a header was added to CASM program.
        if real_pc
            == casm_debug_info
                .sierra_statement_info
                .last()
                .unwrap()
                .end_offset
            && was_run_with_header
        {
            header_and_footer_steps += 1;
            continue;
        }

        if end_of_program_reached {
            unreachable!("End of program reached, but trace continues.");
        }

        current_function_weight += 1;

        // TODO: Maintain a map of pc to sierra statement index (only for PCs we saw), to save lookups.
        let maybe_sierra_statement_idx = sierra_statement_index_by_pc(casm_debug_info, real_pc);
        let sierra_statement_idx = match maybe_sierra_statement_idx {
            MaybeSierraStatementIndexFromPc::SierraStatementIndex(sierra_statement_idx) => {
                sierra_statement_idx
            }
            MaybeSierraStatementIndexFromPc::PcOutOfFunctionArea => {
                continue;
            }
        };

        let user_function_idx =
            user_function_idx_by_sierra_statement_idx(program, sierra_statement_idx);

        let Some(gen_statement) = program.statements.get(sierra_statement_idx.0) else {
            panic!("Failed fetching statement index {}", sierra_statement_idx.0);
        };

        match gen_statement {
            GenStatement::Invocation(invocation) => {
                if matches!(
                    sierra_program_registry.get_libfunc(&invocation.libfunc_id),
                    Ok(CoreConcreteLibfunc::FunctionCall(_))
                ) {
                    // Push to the stack.
                    if function_stack_depth < MAX_TRACE_DEPTH as usize {
                        function_stack.push((user_function_idx, current_function_weight));
                        current_function_weight = 0;
                    }
                    function_stack_depth += 1;
                }
            }
            GenStatement::Return(_) => {
                // Pop from the stack.
                if function_stack_depth <= MAX_TRACE_DEPTH as usize {
                    let current_stack =
                        chain!(function_stack.iter().map(|f| f.0), [user_function_idx])
                            .collect_vec();
                    *stack_trace_weights.entry(current_stack).or_insert(0) +=
                        current_function_weight;

                    let Some((_, caller_function_weight)) = function_stack.pop() else {
                        end_of_program_reached = true;
                        continue;
                    };
                    // Set to the caller function weight to continue counting its cost.
                    current_function_weight = caller_function_weight;
                }
                function_stack_depth -= 1;
            }
        }
    }

    let stack_trace_weights = stack_trace_weights
        .iter()
        .map(|(idx_stack_trace, weight)| {
            Ok((
                index_stack_trace_to_name_stack_trace(program, idx_stack_trace)?,
                *weight,
            ))
        })
        .collect::<Result<_>>()?;

    Ok((stack_trace_weights, header_and_footer_steps))
}

fn index_stack_trace_to_name_stack_trace(
    sierra_program: &Program,
    idx_stack_trace: &[usize],
) -> Result<Vec<FunctionName>> {
    let re_loop_func = Regex::new(r"\[expr\d*\]")
        .context("Failed to create regex normalising loop functions names")?;

    let re_monomorphization = Regex::new(r"<.*>")
        .context("Failed to create regex normalising mononorphised generic functions names")?;

    let stack_with_recursive_functions = idx_stack_trace
        .iter()
        .map(|idx| {
            let sierra_func_name = &sierra_program.funcs[*idx].id.to_string();
            // Remove suffix in case of loop function e.g. `[expr36]`.
            let func_name = re_loop_func.replace(sierra_func_name, "");
            // Remove parameters from monomorphised Cairo generics e.g. `<felt252>`.
            re_monomorphization.replace(&func_name, "").to_string()
        })
        .collect_vec();

    // Consolidate recursive function calls into one function call - they mess up the flame graph.
    let mut result = vec![stack_with_recursive_functions[0].clone()];
    for i in 1..stack_with_recursive_functions.len() {
        if stack_with_recursive_functions[i - 1] != stack_with_recursive_functions[i] {
            result.push(stack_with_recursive_functions[i].clone());
        }
    }

    Ok(result)
}

fn user_function_idx_by_sierra_statement_idx(
    sierra_program: &Program,
    statement_idx: StatementIdx,
) -> usize {
    // The `-1` here can't cause an underflow as the first function's statement id is always 0,
    // so it is always on the left side of the partition, thus the partition index is > 0.
    sierra_program
        .funcs
        .partition_point(|f| f.entry_point.0 <= statement_idx.0)
        - 1
}

fn sierra_statement_index_by_pc(
    casm_debug_info: &CairoProgramDebugInfo,
    pc: usize,
) -> MaybeSierraStatementIndexFromPc {
    // The `-1` here can't cause an underflow as the first statement is always at offset 0,
    // so it is always on the left side of the partition, thus the partition index is > 0.
    let statement_index = StatementIdx(
        casm_debug_info
            .sierra_statement_info
            .partition_point(|x| x.start_offset <= pc)
            - 1,
    );
    if casm_debug_info.sierra_statement_info[statement_index.0].end_offset <= pc {
        MaybeSierraStatementIndexFromPc::PcOutOfFunctionArea
    } else {
        MaybeSierraStatementIndexFromPc::SierraStatementIndex(statement_index)
    }
}
