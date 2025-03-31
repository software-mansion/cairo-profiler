use crate::profiler_config::FunctionLevelConfig;
use crate::trace_reader::function_trace_builder::function_stack_trace::{
    CallStack, VecWithLimitedCapacity,
};
use crate::trace_reader::function_trace_builder::inlining::build_original_call_stack_with_inlined_calls;
use crate::trace_reader::function_trace_builder::stack_trace::{
    map_syscall_to_selector, trace_to_samples,
};
use crate::trace_reader::sample::{FunctionCall, InternalFunctionCall, Sample};
use crate::versioned_constants_reader::VersionedConstants;
use cairo_annotations::annotations::profiler::{FunctionName, ProfilerAnnotationsV1};
use cairo_annotations::trace_data::CasmLevelInfo;
use cairo_annotations::{MappingResult, map_pcs_to_sierra_statement_ids};
use cairo_lang_sierra::extensions::core::{CoreConcreteLibfunc, CoreLibfunc, CoreType};
use cairo_lang_sierra::extensions::starknet::StarkNetConcreteLibfunc;
use cairo_lang_sierra::program::{GenStatement, Program, StatementIdx};
use cairo_lang_sierra::program_registry::ProgramRegistry;
use cairo_lang_sierra_to_casm::compiler::CairoProgramDebugInfo;
use std::collections::HashMap;
use std::ops::AddAssign;

mod function_stack_trace;
mod inlining;
mod stack_trace;

pub struct FunctionLevelProfilingInfo {
    pub functions_samples: Vec<Sample>,
    pub header_resources: ChargedResources,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
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

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct SierraGasConsumed(pub usize);

impl AddAssign for SierraGasConsumed {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl AddAssign<usize> for SierraGasConsumed {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub struct ChargedResources {
    pub steps: Steps,
    pub sierra_gas_consumed: SierraGasConsumed,
}

impl ChargedResources {
    pub fn increment(&mut self, sierra_gas_tracking: bool) {
        if sierra_gas_tracking {
            self.sierra_gas_consumed += 100;
        } else {
            self.steps += 1;
        }
    }
}

/// Collects profiling info of the current run using the trace.
#[expect(clippy::too_many_lines)]
pub fn collect_function_level_profiling_info(
    program: &Program,
    casm_debug_info: &CairoProgramDebugInfo,
    casm_level_info: &CasmLevelInfo,
    statements_functions_map: Option<&ProfilerAnnotationsV1>,
    function_level_config: &FunctionLevelConfig,
    versioned_constants: &VersionedConstants,
    sierra_gas_tracking: bool,
) -> FunctionLevelProfilingInfo {
    let sierra_program_registry = &ProgramRegistry::<CoreType, CoreLibfunc>::new(program).unwrap();

    let mut call_stack = CallStack::new(function_level_config.max_function_stack_trace_depth);

    // The value is the charged resource of the stack trace so far, not including the pending resources being
    // tracked at the time. The key is a function stack trace.
    let mut functions_stack_traces: HashMap<Vec<FunctionCall>, ChargedResources> = HashMap::new();

    // The value is the number of invocations of the syscall in the trace.
    // The key is a syscall stack trace.
    let mut syscall_stack_traces: HashMap<Vec<FunctionCall>, i64> = HashMap::new();

    // Header charged resources are counted separately and then displayed as charged resources
    // of the entrypoint in the profile tree. It is because technically they don't belong
    // to any function, but still increase the number of total steps/gas consumed.
    // The value is different from zero only for functions run with header.
    let mut header_resources = ChargedResources::default();
    let mut end_of_program_reached = false;
    // Syscalls can be recognised by GenStatement::Invocation, but they do not have GenStatement::Return
    // That's why we must track entry to a syscall, and leave as soon as we're out of given GenStatement::Invocation
    let mut in_syscall_idx: Option<StatementIdx> = None;

    let sierra_statements = map_pcs_to_sierra_statement_ids(casm_debug_info, casm_level_info);

    for statement in sierra_statements {
        let sierra_statement_idx = match statement {
            MappingResult::SierraStatementIdx(sierra_statement_idx) => sierra_statement_idx,
            MappingResult::Header => {
                header_resources.increment(sierra_gas_tracking);
                continue;
            }
            MappingResult::PcOutOfFunctionArea => {
                continue;
            }
        };

        if end_of_program_reached {
            unreachable!("End of program reached, but trace continues.");
        }

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
            statements_functions_map,
        );

        functions_stack_traces
            .entry(current_call_stack.clone().into())
            .or_default()
            .increment(sierra_gas_tracking);

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
                                    FunctionName(map_syscall_to_selector(syscall).to_string()),
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
        versioned_constants,
        sierra_gas_tracking,
    );

    FunctionLevelProfilingInfo {
        functions_samples,
        header_resources,
    }
}

fn build_current_call_stack(
    call_stack: &CallStack,
    current_function_name: FunctionName,
    show_inlined_functions: bool,
    sierra_statement_idx: StatementIdx,
    statements_functions_map: Option<&ProfilerAnnotationsV1>,
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
