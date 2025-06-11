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
use cairo_lang_sierra::extensions::gas::CostTokenType;
use cairo_lang_sierra::extensions::starknet::StarkNetConcreteLibfunc;
use cairo_lang_sierra::program::{GenStatement, Program, StatementIdx};
use cairo_lang_sierra::program_registry::ProgramRegistry;
use cairo_lang_sierra_gas::ComputeCostInfoProvider;
use cairo_lang_sierra_gas::core_libfunc_cost_base::core_libfunc_cost;
use cairo_lang_sierra_gas::objects::{BranchCost, ConstCost, PreCost};
use cairo_lang_sierra_to_casm::compiler::CairoProgramDebugInfo;
use std::collections::HashMap;
use std::ops::{AddAssign, SubAssign};

mod function_stack_trace;
mod inlining;
mod stack_trace;

pub struct FunctionLevelProfilingInfo {
    pub functions_samples: Vec<Sample>,
    pub header_resources: ChargedResources,
}

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug)]
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

#[derive(Clone, Copy, Default, Eq, PartialEq, Debug)]
pub struct SierraGasConsumed(pub usize);

impl AddAssign for SierraGasConsumed {
    fn add_assign(&mut self, rhs: Self) {
        self.0 += rhs.0;
    }
}

impl SubAssign for SierraGasConsumed {
    fn sub_assign(&mut self, rhs: Self) {
        self.0 -= rhs.0;
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ChargedResources {
    pub steps: Steps,
    pub sierra_gas_consumed: SierraGasConsumed,
}

impl ChargedResources {
    pub fn increment(&mut self, sierra_gas_tracking: bool) {
        if sierra_gas_tracking {
            self.sierra_gas_consumed += SierraGasConsumed(100);
        } else {
            self.steps += 1;
        }
    }

    pub fn decrement(&mut self, sierra_gas_tracking: bool) {
        if sierra_gas_tracking {
            self.sierra_gas_consumed -= SierraGasConsumed(100);
        } else {
            self.steps -= 1;
        }
    }
}

impl AddAssign for ChargedResources {
    fn add_assign(&mut self, rhs: Self) {
        self.steps += rhs.steps;
        self.sierra_gas_consumed += rhs.sierra_gas_consumed;
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
    let cost_info_provider = ComputeCostInfoProvider::new(program).unwrap();

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
    // similarly to syscalls, used to track entry to a libfunc (based on steps costs)
    // default value is 1, because this is a minimal trace count (ie one trace is one step)
    let mut libfunc_appearance_tracker = 1;

    let builtins_with_cost = [
        CostTokenType::Bitwise,
        CostTokenType::Pedersen,
        CostTokenType::Poseidon,
        CostTokenType::EcOp,
        CostTokenType::AddMod,
        CostTokenType::MulMod,
    ];

    let libfunc_map: HashMap<u64, String> = program
        .libfunc_declarations
        .iter()
        .map(|declaration| {
            (
                declaration.id.id,
                declaration.long_id.generic_id.0.clone().to_string(),
            )
        })
        .collect();

    let sierra_statements = map_pcs_to_sierra_statement_ids(casm_debug_info, casm_level_info);
    // get all sizes to a hashmap, for quicker lookup
    let mut casm_sizes: HashMap<String, i64> = HashMap::new();
    for entry in casm_debug_info.sierra_statement_info.clone() {
        *casm_sizes
            .entry(entry.instruction_idx.to_string())
            .or_default() += i64::try_from(entry.end_offset - entry.start_offset)
            .expect("Failed to convert casm size to i64");
    }

    let mut function_casm_sizes: HashMap<Vec<FunctionCall>, i64> = HashMap::new();

    for statement in sierra_statements {
        let sierra_statement_idx = match statement {
            MappingResult::SierraStatementIdx(sierra_statement_idx) => sierra_statement_idx,
            MappingResult::Header => {
                header_resources.increment(sierra_gas_tracking);
                continue;
            }
            MappingResult::PcOutOfFunctionArea => {
                let current_stack = call_stack.current_call_stack();
                if current_stack.is_empty() {
                    header_resources.increment(sierra_gas_tracking);
                } else {
                    functions_stack_traces
                        .entry(current_stack.clone().into())
                        .or_default()
                        .increment(sierra_gas_tracking);
                }
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
                let libfunc = sierra_program_registry.get_libfunc(&invocation.libfunc_id);

                match libfunc {
                    Ok(CoreConcreteLibfunc::FunctionCall(_)) => {
                        *function_casm_sizes
                            .entry(current_call_stack.clone().into())
                            .or_default() += casm_sizes
                            .get(&sierra_statement_idx.to_string())
                            .unwrap_or(&0);
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
                        // we do not want to profile builtins from syscalls - they have fixed price and are profiled explicitly
                        if in_syscall_idx.is_none() {
                            let mut libfunc_call_stack = current_call_stack.clone();
                            if function_level_config.show_libfuncs {
                                let libfunc_name = libfunc_map
                                    .get(&invocation.libfunc_id.id)
                                    .expect("Failed to find libfunc in map");

                                // todo: hack, fix this abomination
                                // we are subtracting resources accounted to current function from stack
                                functions_stack_traces
                                    .entry(current_call_stack.clone().into())
                                    .or_default()
                                    .decrement(sierra_gas_tracking);

                                // then appending libfunc to said stack
                                libfunc_call_stack.push(FunctionCall::InternalFunctionCall(
                                    InternalFunctionCall::Libfunc(FunctionName(
                                        libfunc_name.to_owned(),
                                    )),
                                ));
                                // and accounting previously subtracted resources to this libfunc
                                functions_stack_traces
                                    .entry(libfunc_call_stack.clone().into())
                                    .or_default()
                                    .increment(sierra_gas_tracking);
                            }

                            // we do not have builtins vm resources costs, we only include them when tracking sierra gas
                            if sierra_gas_tracking {
                                let libfunc_cost =
                                    core_libfunc_cost(libfunc.unwrap(), &cost_info_provider);

                                for branch_cost in &libfunc_cost {
                                    if let BranchCost::Regular {
                                        const_cost,
                                        pre_cost,
                                    } = branch_cost
                                    {
                                        // determine if we are already tracking this specific invocation (libfunc)
                                        // we use steps estimated by `core_libfunc_cost` function to do this
                                        if const_cost.steps == 1
                                            || const_cost.steps <= libfunc_appearance_tracker
                                        {
                                            // if a given invocation "costs" some builtins, sum them
                                            let post_cost = sum_builtins_cost(
                                                const_cost,
                                                pre_cost,
                                                versioned_constants,
                                                builtins_with_cost,
                                            );

                                            // add builtin cost (resources) to current function in stack
                                            *functions_stack_traces
                                                .entry(libfunc_call_stack.clone().into())
                                                .or_default() +=
                                                ChargedResources {
                                                    steps: Default::default(),
                                                    sierra_gas_consumed: SierraGasConsumed(
                                                        usize::try_from(post_cost).expect("Overflow while converting post_cost to usize"),
                                                    ),
                                                };

                                            libfunc_appearance_tracker = 1;
                                        // if an invocation takes more than 1 step, we skip getting its cost for subsequent
                                        // appearances in stack, so we do not wrongly add the resources multiple times
                                        } else if const_cost.steps > libfunc_appearance_tracker {
                                            libfunc_appearance_tracker += 1;
                                        }
                                    }
                                }
                            }
                            *function_casm_sizes
                                .entry(libfunc_call_stack.clone().into())
                                .or_default() += casm_sizes
                                .get(&sierra_statement_idx.to_string())
                                .unwrap_or(&0);
                        }

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
        &function_casm_sizes,
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

    if current_call_stack.is_empty()
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

fn sum_builtins_cost(
    const_cost: &ConstCost,
    pre_cost: &PreCost,
    versioned_constants: &VersionedConstants,
    builtins_with_cost: [CostTokenType; 6],
) -> u64 {
    let mut post_cost: u64 = u64::try_from(const_cost.range_checks)
        .expect("Failed to convert const_cost to u64")
        * versioned_constants
            .os_constants
            .builtin_gas_costs
            .range_check;
    post_cost += u64::try_from(const_cost.range_checks96)
        .expect("Failed to convert const_cost to u64")
        * versioned_constants
            .os_constants
            .builtin_gas_costs
            .range_check96;

    for builtin in &builtins_with_cost {
        if let Some(value) = pre_cost.0.get(builtin) {
            let builtin_cost = match builtin {
                CostTokenType::Bitwise => {
                    versioned_constants.os_constants.builtin_gas_costs.bitwise
                }
                CostTokenType::Pedersen => {
                    versioned_constants.os_constants.builtin_gas_costs.pedersen
                }
                CostTokenType::Poseidon => {
                    versioned_constants.os_constants.builtin_gas_costs.poseidon
                }
                CostTokenType::EcOp => versioned_constants.os_constants.builtin_gas_costs.ecop,
                CostTokenType::AddMod => versioned_constants.os_constants.builtin_gas_costs.add_mod,
                CostTokenType::MulMod => versioned_constants.os_constants.builtin_gas_costs.mul_mod,
                _ => {
                    panic!("Unknown builtin: {builtin:?}")
                }
            };
            post_cost += u64::try_from(*value)
                .expect("Failed to convert builtin count value to u64")
                * builtin_cost;
        }
    }
    post_cost
}
