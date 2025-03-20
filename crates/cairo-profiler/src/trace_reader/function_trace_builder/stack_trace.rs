use crate::trace_reader::function_trace_builder::ChargedResources;
use crate::trace_reader::sample::{FunctionCall, MeasurementUnit, MeasurementValue, Sample};
use crate::versioned_constants_reader::{
    BuiltinGasCosts, VersionedConstants,
};
use cairo_annotations::trace_data::DeprecatedSyscallSelector;
use cairo_lang_sierra::extensions::starknet::StarkNetConcreteLibfunc;
use itertools::Itertools;
use std::collections::HashMap;

pub fn trace_to_samples(
    functions_stack_traces: HashMap<Vec<FunctionCall>, ChargedResources>,
    syscall_stack_traces: HashMap<Vec<FunctionCall>, i64>,
    versioned_constants: &VersionedConstants,
    sierra_gas_tracking: bool,
) -> Vec<Sample> {
    let multiply_resource_by_invocations = |resource: usize, invocations: i64| -> i64 {
        let resource = i64::try_from(resource).expect("Overflow while converting resource to i64");
        resource
            .checked_mul(invocations)
            .expect("Multiplication overflow")
    };

    let mut function_samples = functions_stack_traces
        .into_iter()
        .map(|(call_stack, cr)| {
            let measurements: HashMap<MeasurementUnit, MeasurementValue> = vec![
                (
                    MeasurementUnit::from("steps".to_string()),
                    MeasurementValue(i64::try_from(cr.steps.0).unwrap()),
                ), // measurementy się mają nie pokazywac jeśli 0
                (
                    MeasurementUnit::from("sierra_gas".to_string()),
                    MeasurementValue(i64::try_from(cr.sierra_gas_consumed.0).unwrap()),
                ),
            ]
            .into_iter()
            .collect();

            Sample {
                call_stack,
                measurements,
            }
        })
        .collect_vec();

    let syscall_samples = syscall_stack_traces
        .into_iter()
        .map(|(call_stack, invocations)| {
            let function_name = call_stack.last().unwrap().function_name();
            let resources = versioned_constants
                .os_resources
                .execute_syscalls
                .get(
                    &function_name
                        .0
                        .as_str()
                        .parse()
                        .expect("Unknown syscall found"),
                )
                .expect("Resource map is expected to contain all syscalls");

            let measurements = match sierra_gas_tracking {
                false => {
                    let mut cs_measurements: HashMap<MeasurementUnit, MeasurementValue> = vec![
                        (
                            MeasurementUnit::from("steps".to_string()),
                            MeasurementValue(multiply_resource_by_invocations(
                                resources.n_steps,
                                invocations,
                            )),
                        ),
                        (
                            MeasurementUnit::from("memory_holes".to_string()),
                            MeasurementValue(multiply_resource_by_invocations(
                                resources.n_memory_holes,
                                invocations,
                            )),
                        ),
                    ]
                    .into_iter()
                    .collect();

                    for (builtin, b_count) in &resources.builtin_instance_counter {
                        cs_measurements.insert(
                            MeasurementUnit::from(builtin.to_string()),
                            MeasurementValue(multiply_resource_by_invocations(
                                *b_count,
                                invocations,
                            )),
                        );
                    }
                    cs_measurements
                }
                true => {
                    let from_steps =
                        resources.n_steps * versioned_constants.os_constants.step_gas_cost as usize; //todo popraw
                    let from_memory_holes = resources.n_memory_holes
                        * versioned_constants.os_constants.memory_hole_gas_cost as usize;
                    let from_builtins: usize = resources
                        .builtin_instance_counter
                        .iter()
                        .map(|(builtin, amount)| {
                            *amount
                                * get_builtin_gas_cost(
                                    builtin,
                                    &versioned_constants.os_constants.builtin_gas_costs,
                                )
                                as usize
                        })
                        .sum();

                    let mut total_cost = (from_steps + from_memory_holes + from_builtins) as u64;

                    if total_cost
                        < &versioned_constants
                            .os_constants
                            .syscall_base_gas_cost
                            .step_gas_cost
                            * &versioned_constants.os_constants.step_gas_cost
                    {
                        total_cost = &versioned_constants
                            .os_constants
                            .syscall_base_gas_cost
                            .step_gas_cost
                            * &versioned_constants.os_constants.step_gas_cost;
                    }

                    let cs_measurements: HashMap<MeasurementUnit, MeasurementValue> = vec![(
                        MeasurementUnit::from("sierra_gas".to_string()),
                        MeasurementValue(multiply_resource_by_invocations(
                            total_cost as usize,
                            invocations,
                        )),
                    )]
                    .into_iter()
                    .collect();

                    cs_measurements
                }
            };

            Sample {
                call_stack,
                measurements,
            }
        })
        .collect_vec();

    function_samples.extend(syscall_samples);
    function_samples
}

pub fn map_syscall_to_selector(syscall: &StarkNetConcreteLibfunc) -> DeprecatedSyscallSelector {
    match syscall {
        StarkNetConcreteLibfunc::CallContract(_) => DeprecatedSyscallSelector::CallContract,
        StarkNetConcreteLibfunc::Deploy(_) => DeprecatedSyscallSelector::Deploy,
        StarkNetConcreteLibfunc::EmitEvent(_) => DeprecatedSyscallSelector::EmitEvent,
        StarkNetConcreteLibfunc::GetBlockHash(_) => DeprecatedSyscallSelector::GetBlockHash,
        StarkNetConcreteLibfunc::GetExecutionInfo(_) => DeprecatedSyscallSelector::GetExecutionInfo,
        StarkNetConcreteLibfunc::GetExecutionInfoV2(_) => {
            DeprecatedSyscallSelector::GetExecutionInfo
        }
        StarkNetConcreteLibfunc::Keccak(_) => DeprecatedSyscallSelector::Keccak,
        StarkNetConcreteLibfunc::LibraryCall(_) => DeprecatedSyscallSelector::LibraryCall,
        StarkNetConcreteLibfunc::ReplaceClass(_) => DeprecatedSyscallSelector::ReplaceClass,
        StarkNetConcreteLibfunc::SendMessageToL1(_) => DeprecatedSyscallSelector::SendMessageToL1,
        StarkNetConcreteLibfunc::StorageRead(_) => DeprecatedSyscallSelector::StorageRead,
        StarkNetConcreteLibfunc::StorageWrite(_) => DeprecatedSyscallSelector::StorageWrite,
        StarkNetConcreteLibfunc::Sha256ProcessBlock(_) => {
            DeprecatedSyscallSelector::Sha256ProcessBlock
        }
        //
        _ => panic!("Missing mapping to a syscall"),
    }
}
fn get_builtin_gas_cost(builtin: &str, builtins_costs: &BuiltinGasCosts) -> u64 {
    match builtin {
        "range_check_builtin" => builtins_costs.range_check,
        "range_check96_builtin" => builtins_costs.range_check96,
        "keccak_builtin" => builtins_costs.keccak,
        "pedersen_builtin" => builtins_costs.pedersen,
        "bitwise_builtin" => builtins_costs.bitwise,
        "ec_op_builtin" => builtins_costs.ecop,
        "poseidon_builtin" => builtins_costs.poseidon,
        "add_mod_builtin" => builtins_costs.add_mod,
        "mul_mod_builtin" => builtins_costs.mul_mod,
        "ecdsa_builtin" => builtins_costs.ecdsa,
        _ => panic!("Unknown builtin: {builtin}"),
    }
}
