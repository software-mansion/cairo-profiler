use crate::profiler_config::FunctionLevelConfig;
use crate::trace_reader::function_name::FunctionNameExt;
use crate::trace_reader::function_trace_builder::cost::{CostEntry, ProfilerInvocationInfo};
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
use cairo_annotations::trace_data::{CasmLevelInfo, SummedUpEvent};
use cairo_annotations::{MappingResult, map_pcs_to_sierra_statement_ids};
use cairo_lang_sierra::extensions::core::{CoreConcreteLibfunc, CoreLibfunc, CoreType};
use cairo_lang_sierra::extensions::gas::CostTokenType;
use cairo_lang_sierra::extensions::starknet::StarknetConcreteLibfunc;
use cairo_lang_sierra::program::{GenStatement, Program, StatementIdx};
use cairo_lang_sierra::program_registry::ProgramRegistry;
use cairo_lang_sierra_gas::compute_precost_info;
use cairo_lang_sierra_gas::core_libfunc_cost::core_libfunc_cost;
use cairo_lang_sierra_gas::gas_info::GasInfo;
use cairo_lang_sierra_to_casm::circuit::CircuitsInfo;
use cairo_lang_sierra_to_casm::compiler::CairoProgramDebugInfo;
use cairo_lang_sierra_to_casm::metadata::{Metadata, MetadataComputationConfig, calc_metadata};
use cairo_lang_sierra_type_size::{TypeSizeMap, get_type_size_map};
use cairo_lang_utils::ordered_hash_map::OrderedHashMap;
use std::collections::{HashMap, VecDeque};
use std::ops::{AddAssign, SubAssign};

mod cost;
mod function_stack_trace;
mod inlining;
pub mod stack_trace;

pub struct FunctionLevelProfilingInfo {
    pub functions_samples: Vec<Sample>,
    pub header_resources: ChargedResources,
    /// Call stacks that triggered each `EntryPointCall` in `nested_calls`,
    /// used as the entry context for sample collection.
    pub nested_call_triggers: Vec<Vec<FunctionCall>>,
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
}

impl AddAssign for ChargedResources {
    fn add_assign(&mut self, rhs: Self) {
        self.steps += rhs.steps;
        self.sierra_gas_consumed += rhs.sierra_gas_consumed;
    }
}

pub struct ProgramInfos {
    pub precost_info: GasInfo,
    pub circuits_info: CircuitsInfo,
    pub type_sizes: TypeSizeMap,
    pub metadata: Metadata,
}

/// Collects profiling info of the current run using the trace.
#[expect(clippy::too_many_lines, clippy::too_many_arguments)]
pub fn collect_function_level_profiling_info(
    program: &Program,
    casm_debug_info: &CairoProgramDebugInfo,
    casm_level_info: &CasmLevelInfo,
    statements_functions_map: Option<&ProfilerAnnotationsV1>,
    function_level_config: &FunctionLevelConfig,
    versioned_constants: &VersionedConstants,
    sierra_gas_tracking: bool,
    entrypoint_calldata_lengths: Vec<usize>,
    in_transaction: bool,
    events: &mut VecDeque<SummedUpEvent>,
    cairo_enable_gas: bool,
) -> FunctionLevelProfilingInfo {
    let sierra_program_registry = &ProgramRegistry::<CoreType, CoreLibfunc>::new(program).unwrap();
    let maybe_program_infos =
        cairo_enable_gas.then(|| compute_program_infos(program, sierra_program_registry));

    let mut call_stack = CallStack::new(function_level_config.max_function_stack_trace_depth);

    // The value is the charged resource of the stack trace so far, not including the pending resources being
    // tracked at the time. The key is a function stack trace.
    let mut functions_stack_traces: HashMap<Vec<FunctionCall>, ChargedResources> = HashMap::new();

    // The value is the number of invocations of the syscall in the trace.
    // The key is a syscall stack trace.
    let mut syscall_stack_traces: OrderedHashMap<Vec<FunctionCall>, i64> =
        OrderedHashMap::default();

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
    // default value is 100, because this is a minimal trace cost of a single trace
    // (ie one trace is one step which is 100 sierra gas)
    let mut libfunc_appearance_tracker = 100;
    // To correctly append additional entrypoint calls to the tree, we need to obtain
    // a vector of calls for each `nested_call`. These calls can be one of:
    // Deploy, CallContract, or LibraryCall syscalls
    let mut nested_call_triggers: Vec<Vec<FunctionCall>> = Vec::new();
    // Each EmitEvent syscall should have a corresponding event containing keys and data length
    // taken from trace file. Later on the data is used to estimate l2 gas cost of this syscall.
    let mut events_map: HashMap<Vec<FunctionCall>, Vec<SummedUpEvent>> = HashMap::new();

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

        let Some(gen_statement) = program.statements.get(sierra_statement_idx.0) else {
            panic!("Failed fetching statement index {}", sierra_statement_idx.0);
        };

        let profiler_info_provider =
            maybe_program_infos
                .as_ref()
                .map(|infos| ProfilerInvocationInfo {
                    type_sizes: &infos.type_sizes,
                    circuits_info: &infos.circuits_info,
                    metadata: &infos.metadata,
                    idx: sierra_statement_idx,
                });

        match gen_statement {
            GenStatement::Invocation(invocation) => {
                let libfunc = sierra_program_registry.get_libfunc(&invocation.libfunc_id);

                match libfunc {
                    Ok(CoreConcreteLibfunc::FunctionCall(_)) => {
                        increment_resource(
                            &mut functions_stack_traces,
                            &current_call_stack,
                            sierra_gas_tracking,
                        );
                        *function_casm_sizes
                            .entry(current_call_stack.clone().into())
                            .or_default() += casm_sizes
                            .get(&sierra_statement_idx.to_string())
                            .unwrap_or(&0);
                        call_stack.enter_function_call(current_call_stack);
                    }
                    Ok(CoreConcreteLibfunc::Starknet(libfunc)) => {
                        increment_resource(
                            &mut functions_stack_traces,
                            &current_call_stack,
                            sierra_gas_tracking,
                        );
                        let syscall = match libfunc {
                            StarknetConcreteLibfunc::CallContract(_)
                            | StarknetConcreteLibfunc::Deploy(_)
                            | StarknetConcreteLibfunc::EmitEvent(_)
                            | StarknetConcreteLibfunc::GetBlockHash(_)
                            | StarknetConcreteLibfunc::GetExecutionInfo(_)
                            | StarknetConcreteLibfunc::GetExecutionInfoV2(_)
                            | StarknetConcreteLibfunc::Keccak(_)
                            | StarknetConcreteLibfunc::LibraryCall(_)
                            | StarknetConcreteLibfunc::ReplaceClass(_)
                            | StarknetConcreteLibfunc::SendMessageToL1(_)
                            | StarknetConcreteLibfunc::StorageRead(_)
                            | StarknetConcreteLibfunc::StorageWrite(_)
                            | StarknetConcreteLibfunc::Sha256ProcessBlock(_)
                            | StarknetConcreteLibfunc::MetaTxV0(_) => libfunc,
                            _ => {
                                if in_syscall_idx.is_some() {
                                    in_syscall_idx = None;
                                }
                                continue;
                            }
                        };
                        if in_syscall_idx.as_ref() != Some(&sierra_statement_idx) {
                            register_syscall(
                                syscall,
                                sierra_statement_idx,
                                &mut in_syscall_idx,
                                &current_call_stack,
                                &mut nested_call_triggers,
                                &mut syscall_stack_traces,
                                &mut events_map,
                                events,
                            );
                        }
                    }
                    _ => {
                        // we do not want to profile builtins from syscalls - they have fixed price and are profiled explicitly
                        if in_syscall_idx.is_none() {
                            let libfunc_name = if function_level_config.show_libfuncs {
                                Some(
                                    libfunc_map
                                        .get(&invocation.libfunc_id.id)
                                        .expect("Failed to find libfunc in map")
                                        .as_str(),
                                )
                            } else {
                                None
                            };

                            let effective_stack =
                                effective_call_stack(&current_call_stack, libfunc_name);

                            increment_resource(
                                &mut functions_stack_traces,
                                &effective_stack,
                                sierra_gas_tracking,
                            );

                            // We can only calculate a libfunc additional cost when tracking sierra
                            // and when gas was enabled during cairo compilation, otherwise the info
                            // cannot be obtained at all
                            if sierra_gas_tracking && cairo_enable_gas {
                                let precost_info =
                                    &maybe_program_infos.as_ref().unwrap().precost_info;
                                let cost_vector = core_libfunc_cost(
                                    precost_info,
                                    &sierra_statement_idx,
                                    libfunc.expect("fatal: expected libfunc, but did not found in sierra registry"),
                                    &profiler_info_provider.expect("fatal: enable-gas was set in cairo, but program infos is unavailable!"),
                                );

                                add_builtin_sierra_costs(
                                    &cost_vector,
                                    &mut libfunc_appearance_tracker,
                                    versioned_constants,
                                    &mut functions_stack_traces,
                                    &effective_stack,
                                );
                            }
                            *function_casm_sizes
                                .entry(effective_stack.clone().into())
                                .or_default() += casm_sizes
                                .get(&sierra_statement_idx.to_string())
                                .unwrap_or(&0);
                        } else {
                            increment_resource(
                                &mut functions_stack_traces,
                                &current_call_stack,
                                sierra_gas_tracking,
                            );
                        }

                        // If we were in a syscall this is the time we go out of it, as pcs no longer
                        // belong to GenStatement::Invocation of CoreConcreteLibfunc::StarkNet
                        in_syscall_idx = None;
                    }
                }
            }
            GenStatement::Return(_) => {
                increment_resource(
                    &mut functions_stack_traces,
                    &current_call_stack,
                    sierra_gas_tracking,
                );
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
        entrypoint_calldata_lengths,
        in_transaction && cairo_enable_gas,
        &events_map,
    );

    FunctionLevelProfilingInfo {
        functions_samples,
        header_resources,
        nested_call_triggers,
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

fn sum_builtins_cost(branch_cost: &CostEntry, versioned_constants: &VersionedConstants) -> u64 {
    let mut post_cost: u64 = u64::default();

    for (token, cost) in branch_cost.iter() {
        let cost = u64::try_from(cost.max(0)).expect("Cost must be non-negative after clamping");
        let builtin_cost = match token {
            CostTokenType::Pedersen => versioned_constants.os_constants.builtin_gas_costs.pedersen,
            CostTokenType::Poseidon => versioned_constants.os_constants.builtin_gas_costs.poseidon,
            CostTokenType::Bitwise => versioned_constants.os_constants.builtin_gas_costs.bitwise,
            CostTokenType::EcOp => versioned_constants.os_constants.builtin_gas_costs.ecop,
            CostTokenType::AddMod => versioned_constants.os_constants.builtin_gas_costs.add_mod,
            CostTokenType::MulMod => versioned_constants.os_constants.builtin_gas_costs.mul_mod,
            _ => continue,
        };
        post_cost += builtin_cost * cost;
    }
    post_cost
}

fn increment_resource(
    functions_stack_traces: &mut HashMap<Vec<FunctionCall>, ChargedResources>,
    call_stack: &VecWithLimitedCapacity<FunctionCall>,
    sierra_gas_tracking: bool,
) {
    functions_stack_traces
        .entry(call_stack.clone().into())
        .or_default()
        .increment(sierra_gas_tracking);
}

fn effective_call_stack(
    current_stack: &VecWithLimitedCapacity<FunctionCall>,
    libfunc_name: Option<&str>,
) -> VecWithLimitedCapacity<FunctionCall> {
    match libfunc_name {
        Some(name) => {
            let mut stack = current_stack.clone();
            stack.push(FunctionCall::InternalFunctionCall(
                InternalFunctionCall::Libfunc(FunctionName(name.to_owned())),
            ));
            stack
        }
        None => current_stack.clone(),
    }
}

#[expect(clippy::too_many_arguments)]
fn register_syscall(
    syscall: &StarknetConcreteLibfunc,
    sierra_statement_idx: StatementIdx,
    in_syscall_idx: &mut Option<StatementIdx>,
    current_call_stack: &VecWithLimitedCapacity<FunctionCall>,
    nested_call_triggers: &mut Vec<Vec<FunctionCall>>,
    syscall_stack_traces: &mut OrderedHashMap<Vec<FunctionCall>, i64>,
    events_map: &mut HashMap<Vec<FunctionCall>, Vec<SummedUpEvent>>,
    events: &mut VecDeque<SummedUpEvent>,
) {
    *in_syscall_idx = Some(sierra_statement_idx);

    let mut current_call_stack_with_syscall = current_call_stack.clone();

    current_call_stack_with_syscall.push(FunctionCall::InternalFunctionCall(
        InternalFunctionCall::Syscall(FunctionName(map_syscall_to_selector(syscall).to_string())),
    ));

    match syscall {
        StarknetConcreteLibfunc::Deploy(_)
        | StarknetConcreteLibfunc::CallContract(_)
        | StarknetConcreteLibfunc::LibraryCall(_) => {
            nested_call_triggers.push(current_call_stack_with_syscall.clone().into());
        }
        StarknetConcreteLibfunc::EmitEvent(_) => {
            if !events.is_empty() {
                events_map
                    .entry(current_call_stack_with_syscall.clone().into())
                    .or_default()
                    .push(events.front().unwrap().clone());
                events.pop_front();
            }
        }
        _ => {}
    }

    *syscall_stack_traces
        .entry(current_call_stack_with_syscall.into())
        .or_insert(0) += 1;
}

fn add_builtin_sierra_costs(
    cost_vector: &Vec<OrderedHashMap<CostTokenType, i64>>,
    libfunc_appearance_tracker: &mut i64,
    versioned_constants: &VersionedConstants,
    functions_stack_traces: &mut HashMap<Vec<FunctionCall>, ChargedResources>,
    libfunc_call_stack: &VecWithLimitedCapacity<FunctionCall>,
) {
    for branch_cost_map in cost_vector {
        let branch_cost = CostEntry::from_map(branch_cost_map);

        // determine if we are already tracking this specific invocation (libfunc)
        // we use sierra gas estimated by `core_libfunc_cost` function to do this
        if branch_cost.konst == 100 || branch_cost.konst <= *libfunc_appearance_tracker {
            // if a given invocation "costs" some builtins, sum them
            let post_cost = sum_builtins_cost(&branch_cost, versioned_constants);

            // add builtin cost (resources) to current function in stack
            *functions_stack_traces
                .entry(libfunc_call_stack.clone().into())
                .or_default() += ChargedResources {
                steps: Steps::default(),
                sierra_gas_consumed: SierraGasConsumed(
                    usize::try_from(post_cost)
                        .expect("Overflow while converting post_cost to usize"),
                ),
            };

            *libfunc_appearance_tracker = 100;
        // if an invocation takes more than 1 step (100 sierra gas), we skip getting its cost for subsequent
        // appearances in stack, so we do not wrongly add the resources multiple times
        } else if branch_cost.konst > *libfunc_appearance_tracker {
            *libfunc_appearance_tracker += 100;
        }
    }
}

fn compute_program_infos(
    program: &Program,
    sierra_program_registry: &ProgramRegistry<CoreType, CoreLibfunc>,
) -> ProgramInfos {
    let precost_info = compute_precost_info(program).expect("Failed to compute pre-cost info");
    let circuits_info = CircuitsInfo::new(
        sierra_program_registry,
        program.type_declarations.iter().map(|td| &td.id),
    )
    .expect("Failed to compute circuits info");
    let type_sizes =
        get_type_size_map(program, sierra_program_registry).expect("Failed to get type-size map");
    let metadata = calc_metadata(program, MetadataComputationConfig::default())
        .expect("Failed to compute metadata");

    ProgramInfos {
        precost_info,
        circuits_info,
        type_sizes,
        metadata,
    }
}
