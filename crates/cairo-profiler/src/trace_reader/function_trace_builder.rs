use crate::profiler_config::FunctionLevelConfig;
use crate::sierra_loader::StatementsFunctionsMap;
use crate::trace_reader::function_name::FunctionName;
use crate::trace_reader::function_trace_builder::function_stack_trace::{
    CallStack, VecWithLimitedCapacity,
};
use crate::trace_reader::function_trace_builder::inlining::build_original_call_stack_with_inlined_calls;
use crate::trace_reader::function_trace_builder::stack_trace::{
    map_syscall_to_selector, trace_to_samples,
};
use crate::trace_reader::sample::{FunctionCall, InternalFunctionCall, Sample};
use crate::versioned_constants_reader::OsResources;
use cairo_lang_sierra::extensions::core::{CoreConcreteLibfunc, CoreLibfunc, CoreType};
use cairo_lang_sierra::extensions::starknet::StarkNetConcreteLibfunc;
use cairo_lang_sierra::program::{GenStatement, Program, StatementIdx};
use cairo_lang_sierra::program_registry::ProgramRegistry;
use cairo_lang_sierra_to_casm::compiler::CairoProgramDebugInfo;
use std::collections::HashMap;
use std::ops::AddAssign;
use trace_data::TraceEntry;

mod function_stack_trace;
mod inlining;
mod stack_trace;

pub struct FunctionLevelProfilingInfo {
    pub functions_samples: Vec<Sample>,
    pub header_steps: Steps,
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

enum MaybeSierraStatementIndex {
    SierraStatementIndex(StatementIdx),
    PcOutOfFunctionArea,
}

/// Collects profiling info of the current run using the trace.
#[allow(clippy::too_many_lines)]
pub fn collect_function_level_profiling_info(
    trace: &[TraceEntry],
    program: &Program,
    casm_debug_info: &CairoProgramDebugInfo,
    run_with_call_header: bool,
    statements_functions_map: &Option<StatementsFunctionsMap>,
    function_level_config: &FunctionLevelConfig,
    os_resources_map: &OsResources,
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
    let real_minimal_pc = if run_with_call_header {
        trace.last().unwrap().pc + 1
    } else {
        1
    };

    let mut call_stack = CallStack::new(function_level_config.max_function_stack_trace_depth);

    // The value is the steps of the stack trace so far, not including the pending steps being
    // tracked at the time. The key is a function stack trace.
    let mut functions_stack_traces: HashMap<Vec<FunctionCall>, Steps> = HashMap::new();

    // The value is the number of invocations of the syscall in the trace.
    // The key is a syscall stack trace.
    let mut syscall_stack_traces: HashMap<Vec<FunctionCall>, i64> = HashMap::new();

    // Header steps are counted separately and then displayed as steps of the entrypoint in the
    // profile tree. It is because technically they don't belong to any function, but still increase
    // the number of total steps. The value is different from zero only for functions run with header.
    let mut header_steps = Steps(0);
    let mut end_of_program_reached = false;
    // Syscalls can be recognised by GenStatement::Invocation but they do not have GenStatement::Return
    // That's why we must track entry to a syscall, and leave as soon as we're out of given GenStatement::Invocation
    let mut in_syscall_idx: Option<StatementIdx> = None;

    for step in trace {
        // Skip the header.
        if step.pc < real_minimal_pc {
            header_steps += 1;
            continue;
        }
        // The real pc is equal to (1 + step.pc - real_minimal_pc). This difference however
        // is a code offset of the real pc. We need real code offset to map pc to sierra statement.
        let real_pc_code_offset = step.pc - real_minimal_pc;

        if end_of_program_reached {
            unreachable!("End of program reached, but trace continues.");
        }

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

        let current_call_stack = build_current_call_stack(
            &call_stack,
            current_function_name,
            function_level_config.show_inlined_functions,
            sierra_statement_idx,
            statements_functions_map.as_ref(),
        );

        *functions_stack_traces
            .entry(current_call_stack.clone().into())
            .or_insert(Steps(0)) += 1;

        let Some(gen_statement) = program.statements.get(sierra_statement_idx.0) else {
            panic!("Failed fetching statement index {}", sierra_statement_idx.0);
        };

        match gen_statement {
            GenStatement::Invocation(invocation) => {
                match sierra_program_registry.get_libfunc(&invocation.libfunc_id) {
                    Ok(CoreConcreteLibfunc::FunctionCall(_)) => {
                        call_stack.enter_function_call(current_call_stack);
                    }
                    Ok(CoreConcreteLibfunc::StarkNet(libfunc)) => {
                        let syscall = match libfunc {
                            StarkNetConcreteLibfunc::CallContract(_)
                            | StarkNetConcreteLibfunc::Deploy(_)
                            | StarkNetConcreteLibfunc::EmitEvent(_)
                            | StarkNetConcreteLibfunc::GetBlockHash(_)
                            | StarkNetConcreteLibfunc::GetExecutionInfo(_)
                            | StarkNetConcreteLibfunc::GetExecutionInfoV2(_)
                            | StarkNetConcreteLibfunc::Keccak(_)
                            | StarkNetConcreteLibfunc::LibraryCall(_)
                            | StarkNetConcreteLibfunc::ReplaceClass(_)
                            | StarkNetConcreteLibfunc::SendMessageToL1(_)
                            | StarkNetConcreteLibfunc::StorageRead(_)
                            | StarkNetConcreteLibfunc::StorageWrite(_)
                            | StarkNetConcreteLibfunc::Sha256ProcessBlock(_) => libfunc,
                            _ => {
                                if in_syscall_idx.is_some() {
                                    in_syscall_idx = None;
                                }
                                continue;
                            }
                        };
                        if in_syscall_idx.as_ref() != Some(&sierra_statement_idx) {
                            in_syscall_idx = Some(sierra_statement_idx);

                            let mut current_call_stack_with_syscall = current_call_stack.clone();
                            current_call_stack_with_syscall.push(
                                FunctionCall::InternalFunctionCall(InternalFunctionCall::Syscall(
                                    FunctionName(map_syscall_to_selector(syscall).into()),
                                )),
                            );
                            *syscall_stack_traces
                                .entry(current_call_stack_with_syscall.clone().into())
                                .or_insert(0) += 1;
                        }
                    }
                    _ => {
                        // If we were in a syscall this is the time we go out of it, as pcs no longer
                        // belong to GenStatement::Invocation of CoreConcreteLibfunc::StarkNet
                        in_syscall_idx = None;
                    }
                }
            }
            GenStatement::Return(_) => {
                if call_stack.exit_function_call().is_none() {
                    end_of_program_reached = true;
                }
            }
        }
    }

    let functions_samples = trace_to_samples(
        functions_stack_traces,
        syscall_stack_traces,
        os_resources_map,
    );

    FunctionLevelProfilingInfo {
        functions_samples,
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

fn build_current_call_stack(
    call_stack: &CallStack,
    current_function_name: FunctionName,
    show_inlined_functions: bool,
    sierra_statement_idx: StatementIdx,
    statements_functions_map: Option<&StatementsFunctionsMap>,
) -> VecWithLimitedCapacity<FunctionCall> {
    let mut current_call_stack = call_stack.current_call_stack().clone();

    if current_call_stack.len() == 0
        || *current_call_stack[current_call_stack.len() - 1].function_name()
            != current_function_name
    {
        current_call_stack.push(FunctionCall::InternalFunctionCall(
            InternalFunctionCall::NonInlined(current_function_name),
        ));
    }

    if show_inlined_functions {
        build_original_call_stack_with_inlined_calls(
            sierra_statement_idx,
            statements_functions_map,
            current_call_stack,
        )
    } else {
        current_call_stack
    }
}
