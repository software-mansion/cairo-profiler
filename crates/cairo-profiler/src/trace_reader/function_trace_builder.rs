use crate::profiler_config::FunctionLevelConfig;
use crate::sierra_loader::StatementsFunctionsMap;
use crate::trace_reader::function_name::FunctionName;
use crate::trace_reader::function_trace_builder::function_call_trace::{
    FunctionStack, FunctionType,
};
use crate::trace_reader::function_trace_builder::inlining::add_inlined_functions_info;
use crate::trace_reader::Function;
use cairo_lang_sierra::extensions::core::{CoreConcreteLibfunc, CoreLibfunc, CoreType};
use cairo_lang_sierra::program::{GenStatement, Program, StatementIdx};
use cairo_lang_sierra::program_registry::ProgramRegistry;
use cairo_lang_sierra_to_casm::compiler::CairoProgramDebugInfo;
use itertools::{chain, Itertools};
use std::collections::HashMap;
use std::ops::{AddAssign, SubAssign};
use trace_data::TraceEntry;

mod function_call_trace;
mod inlining;

pub struct FunctionLevelProfilingInfo {
    pub functions_traces: Vec<FunctionCallTrace>,
    pub header_steps: Steps,
}

pub struct FunctionCallTrace {
    pub call_trace: Vec<Function>,
    pub steps: Steps,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub struct Steps(pub usize);

impl AddAssign for Steps {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl AddAssign<usize> for Steps {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl SubAssign<usize> for Steps {
    fn sub_assign(&mut self, rhs: usize) {
        self.0 -= rhs;
    }
}

enum MaybeSierraStatementIndex {
    SierraStatementIndex(StatementIdx),
    PcOutOfFunctionArea,
}

/// Collects profiling info of the current run using the trace.
pub fn collect_function_level_profiling_info(
    trace: &[TraceEntry],
    program: &Program,
    casm_debug_info: &CairoProgramDebugInfo,
    was_run_with_header: bool,
    maybe_statements_functions_map: &Option<StatementsFunctionsMap>,
    function_level_config: &FunctionLevelConfig,
) -> FunctionLevelProfilingInfo {
    let sierra_program_registry = &ProgramRegistry::<CoreType, CoreLibfunc>::new(program).unwrap();

    // Some CASM programs starts with a header of instructions to wrap the real program.
    // `real_minimal_pc` is the PC in the trace that points to the same CASM instruction which would
    // be in the PC=1 in the original CASM program.
    // This is the same as the PC of the last trace entry plus 1, as the header is built to have
    // a `ret` last instruction, which must be the last in the trace of any execution.
    // The first instruction after that is the first instruction in the original CASM program.
    // This logic only applies when a header was added to the CASM program, otherwise the
    // `real_minimal_pc` is the default one which is 1.
    let real_minimal_pc = if was_run_with_header {
        trace.last().unwrap().pc + 1
    } else {
        1
    };

    let mut function_stack = FunctionStack::new(function_level_config.max_function_trace_depth);

    // The value is the steps of the call trace so far, not including the pending steps being
    // tracked at the time. The key is a function call trace.
    let mut functions_traces: HashMap<Vec<Function>, Steps> = HashMap::new();

    // Header steps are counted separately and then displayed as steps of the entrypoint in the
    // profile tree. It is because technically they don't belong to any function, but still increase
    // the number of total steps. The value is different from zero only for functions run with header.
    let mut header_steps = Steps(0);
    let mut current_function_steps = Steps(0);
    let mut end_of_program_reached = false;

    for step in trace {
        // Skip the header. This only makes sense when a header was added to CASM program.
        if step.pc < real_minimal_pc && was_run_with_header {
            header_steps += 1;
            continue;
        }
        // The real pc is equal to (1 + step.pc - real_minimal_pc). This difference however
        // is a code offset of the real pc. We need real code offset to map pc to sierra statement.
        let real_pc_code_offset = step.pc - real_minimal_pc;

        if end_of_program_reached {
            unreachable!("End of program reached, but trace continues.");
        }

        current_function_steps += 1;

        let maybe_sierra_statement_idx =
            maybe_sierra_statement_index_by_pc(casm_debug_info, real_pc_code_offset);
        let sierra_statement_idx = match maybe_sierra_statement_idx {
            MaybeSierraStatementIndex::SierraStatementIndex(sierra_statement_idx) => {
                sierra_statement_idx
            }
            MaybeSierraStatementIndex::PcOutOfFunctionArea => {
                continue;
            }
        };

        let current_function_name = FunctionName::from_sierra_statement_idx(
            sierra_statement_idx,
            program,
            function_level_config.split_generics,
        );

        if function_level_config.show_inlined_functions {
            add_inlined_functions_info(
                sierra_statement_idx,
                maybe_statements_functions_map.as_ref(),
                &function_stack,
                &current_function_name,
                &mut functions_traces,
                &mut current_function_steps,
            );
        }

        let Some(gen_statement) = program.statements.get(sierra_statement_idx.0) else {
            panic!("Failed fetching statement index {}", sierra_statement_idx.0);
        };

        match gen_statement {
            GenStatement::Invocation(invocation) => {
                if matches!(
                    sierra_program_registry.get_libfunc(&invocation.libfunc_id),
                    Ok(CoreConcreteLibfunc::FunctionCall(_))
                ) {
                    function_stack
                        .enter_function_call(current_function_name, &mut current_function_steps);
                }
            }
            GenStatement::Return(_) => {
                if let Some(exited_function) = function_stack.exit_function_call() {
                    if let FunctionType::Regular(function) = exited_function {
                        let current_function_names_call_stack = chain!(
                            function_stack.current_function_names_stack(),
                            [function.name, current_function_name]
                        )
                        .collect_vec();

                        let current_function_call_stack = current_function_names_call_stack
                            .into_iter()
                            .map(Function::NonInlined)
                            .collect();

                        *functions_traces
                            .entry(current_function_call_stack)
                            .or_insert(Steps(0)) += current_function_steps;
                        // Set to the caller function steps to continue counting its cost.
                        current_function_steps = function.steps;
                    }
                } else {
                    end_of_program_reached = true;

                    let current_function_stack = vec![Function::NonInlined(current_function_name)];

                    *functions_traces
                        .entry(current_function_stack)
                        .or_insert(Steps(0)) += current_function_steps;
                }
            }
        }
    }

    let functions_traces = functions_traces
        .into_iter()
        .map(|(call_trace, steps)| FunctionCallTrace { call_trace, steps })
        .collect_vec();

    FunctionLevelProfilingInfo {
        functions_traces,
        header_steps,
    }
}

fn maybe_sierra_statement_index_by_pc(
    casm_debug_info: &CairoProgramDebugInfo,
    real_pc_code_offset: usize,
) -> MaybeSierraStatementIndex {
    // The `-1` here can't cause an underflow as the first statement's start offset is always 0,
    // so it is always on the left side of the partition, thus the partition index is > 0.
    let statement_index = StatementIdx(
        casm_debug_info
            .sierra_statement_info
            .partition_point(|x| x.start_offset <= real_pc_code_offset)
            - 1,
    );
    // End offset is exclusive and the casm debug info is sorted in non-descending order by both
    // end offset and start offset. Therefore, the end offset of the last element in that vector
    // is the bytecode length.
    let bytecode_length = casm_debug_info
        .sierra_statement_info
        .last()
        .unwrap()
        .end_offset;
    // If offset is greater or equal the bytecode length it means that it is the outside ret used
    // for e.g. getting pointer to builtins costs table, const segments etc.
    if real_pc_code_offset >= bytecode_length {
        MaybeSierraStatementIndex::PcOutOfFunctionArea
    } else {
        MaybeSierraStatementIndex::SierraStatementIndex(statement_index)
    }
}
