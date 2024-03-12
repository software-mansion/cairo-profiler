use cairo_lang_sierra::extensions::core::{CoreConcreteLibfunc, CoreLibfunc, CoreType};
use cairo_lang_sierra::program::{GenStatement, Program, StatementIdx};
use cairo_lang_sierra::program_registry::ProgramRegistry;
use cairo_lang_sierra_to_casm::compiler::CairoProgramDebugInfo;
use itertools::chain;
use std::collections::HashMap;
use trace_data::TraceEntry;

pub fn collect_profiling_info(
    trace: &[TraceEntry],
    casm_debug_info: &CairoProgramDebugInfo,
    sierra_program: &Program,
    sierra_program_registry: &ProgramRegistry<CoreType, CoreLibfunc>,
) -> Vec<(Vec<String>, usize)> {
    let max_stack_trace_depth = 100;

    let bytecode_len = casm_debug_info
        .sierra_statement_info
        .last()
        .unwrap()
        .code_offset;
    // The CASM program starts with a header of instructions to wrap the real program.
    // `real_pc_0` is the PC in the trace that points to the same CASM instruction which is in
    // the real PC=0 in the original CASM program. That is, all trace's PCs need to be
    // subtracted by `real_pc_0` to get the real PC they point to in the original CASM
    // program.
    // This is the same as the PC of the last trace entry plus 1, as the header is built to have
    // a `ret` last instruction, which must be the last in the trace of any execution.
    // The first instruction after that is the first instruction in the original CASM program.
    let real_pc_0 = trace.last().unwrap().pc + 1;

    // The function stack trace of the current function, excluding the current function (that
    // is, the stack of the caller). Represented as a vector of indices of the functions
    // in the stack (indices of the functions according to the list in the sierra program).
    // Limited to depth `max_stack_trace_depth`. Note `function_stack_depth` tracks the real
    // depth, even if >= `max_stack_trace_depth`.
    let mut function_stack = Vec::new();
    // Tracks the depth of the function stack, without limit. This is usually equal to
    // `function_stack.len()`, but if the actual stack is deeper than `max_stack_trace_depth`,
    // this remains reliable while `function_stack` does not.
    let mut function_stack_depth = 0;
    let mut cur_weight = 0;
    // The key is a function stack trace (see `function_stack`, but including the current
    // function).
    // The value is the weight of the stack trace so far, not including the pending weight being
    // tracked at the time.
    let mut stack_trace_weights = HashMap::new();
    let mut end_of_program_reached = false;
    // The total weight of each Sierra statement.
    // Note the header and footer (CASM instructions added for running the program by the
    // runner). The header is not counted, and the footer is, but then the relevant
    // entry is removed.
    let mut header_footer_steps = 0;

    for step in trace {
        // Skip the header.
        if step.pc < real_pc_0 {
            header_footer_steps += 1;
            continue;
        }
        let real_pc = step.pc - real_pc_0;
        // Skip the footer.
        if real_pc == bytecode_len {
            header_footer_steps += 1;
            continue;
        }

        if end_of_program_reached {
            unreachable!("End of program reached, but trace continues.");
        }

        cur_weight += 1;

        // trace + Sierra / (USC) -> CASM
        // trace + CASM + mapping Cairo func name -> Sierra idx + mapping sierra -> CASM (sierra_idx -> pc)

        // TODO(yuval): Maintain a map of pc to sierra statement index (only for PCs we saw), to
        // save lookups.
        let sierra_statement_idx = StatementIdx(
            casm_debug_info
                .sierra_statement_info
                .partition_point(|x| x.code_offset <= real_pc)
                - 1,
        );
        let user_function_idx =
            user_function_idx_by_sierra_statement_idx(sierra_program, sierra_statement_idx);

        let Some(gen_statement) = sierra_program.statements.get(sierra_statement_idx.0) else {
            panic!("Failed fetching statement index {}", sierra_statement_idx.0);
        };

        match gen_statement {
            GenStatement::Invocation(invocation) => {
                if matches!(
                    sierra_program_registry.get_libfunc(&invocation.libfunc_id),
                    Ok(CoreConcreteLibfunc::FunctionCall(_))
                ) {
                    // Push to the stack.
                    if function_stack_depth < max_stack_trace_depth {
                        function_stack.push((user_function_idx, cur_weight));
                        cur_weight = 0;
                    }
                    function_stack_depth += 1;
                }
            }
            GenStatement::Return(_) => {
                // Pop from the stack.
                if function_stack_depth <= max_stack_trace_depth {
                    // The current stack trace, including the current function.
                    let cur_stack: Vec<_> =
                        chain!(function_stack.iter().map(|f| f.0), [user_function_idx]).collect();
                    *stack_trace_weights.entry(cur_stack).or_insert(0) += cur_weight;

                    let Some(popped) = function_stack.pop() else {
                        // End of the program.
                        end_of_program_reached = true;
                        continue;
                    };
                    cur_weight += popped.1;
                }
                function_stack_depth -= 1;
            }
        }
    }

    *stack_trace_weights
        .iter_mut()
        .find(|(trace, _)| trace.len() == 1)
        .unwrap()
        .1 += header_footer_steps;

    let stack_trace_weights = stack_trace_weights
        .iter()
        .map(|(idx_stack_trace, weight)| {
            (
                index_stack_trace_to_name_stack_trace(sierra_program, idx_stack_trace),
                *weight,
            )
        })
        .collect();

    stack_trace_weights
}

fn index_stack_trace_to_name_stack_trace(
    sierra_program: &Program,
    idx_stack_trace: &[usize],
) -> Vec<String> {
    idx_stack_trace
        .iter()
        .map(|idx| sierra_program.funcs[*idx].id.to_string())
        .collect()
}

// copied from cairo_lang_runner to avoid adding additional dependencies
fn user_function_idx_by_sierra_statement_idx(
    sierra_program: &Program,
    statement_idx: StatementIdx,
) -> usize {
    // The `-1` here can't cause an underflow as the first function's entry point is
    // always 0, so it is always on the left side of the partition, and thus the
    // partition index is >0.
    sierra_program
        .funcs
        .partition_point(|f| f.entry_point.0 <= statement_idx.0)
        - 1
}
