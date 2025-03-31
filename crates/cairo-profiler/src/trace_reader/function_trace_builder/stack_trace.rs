use crate::trace_reader::function_trace_builder::ChargedResources;
use crate::trace_reader::sample::{FunctionCall, MeasurementUnit, MeasurementValue, Sample};
use crate::versioned_constants_reader::{BuiltinGasCosts, VersionedConstants};
use cairo_annotations::trace_data::{DeprecatedSyscallSelector, VmExecutionResources};
use cairo_lang_sierra::extensions::starknet::StarkNetConcreteLibfunc;
use std::collections::HashMap;

pub fn trace_to_samples(
    functions_stack_traces: HashMap<Vec<FunctionCall>, ChargedResources>,
    syscall_stack_traces: HashMap<Vec<FunctionCall>, i64>,
    versioned_constants: &VersionedConstants,
    sierra_gas_tracking: bool,
) -> Vec<Sample> {
    let function_samples: Vec<Sample> = functions_stack_traces
        .into_iter()
        .map(|(call_stack, cr)| map_function_trace_to_sample(call_stack, cr))
        .collect();

    let syscall_samples: Vec<Sample> = syscall_stack_traces
        .into_iter()
        .map(|(call_stack, invocations)| {
            map_syscall_trace_to_sample(
                call_stack,
                invocations,
                versioned_constants,
                sierra_gas_tracking,
            )
        })
        .collect();

    [function_samples, syscall_samples].concat()
}

fn map_function_trace_to_sample(call_stack: Vec<FunctionCall>, cr: ChargedResources) -> Sample {
    let measurements: HashMap<MeasurementUnit, MeasurementValue> = vec![
        (
            MeasurementUnit::from("steps".to_string()),
            MeasurementValue(i64::try_from(cr.steps.0).unwrap()),
        ),
        (
            MeasurementUnit::from("sierra_gas".to_string()),
            MeasurementValue(i64::try_from(cr.sierra_gas_consumed.0).unwrap()),
        ),
    ]
    .into_iter()
    .filter(|(_, value)| *value != 0)
    .collect();

    Sample {
        call_stack,
        measurements,
    }
}

fn map_syscall_trace_to_sample(
    call_stack: Vec<FunctionCall>,
    invocations: i64,
    versioned_constants: &VersionedConstants,
    sierra_gas_tracking: bool,
) -> Sample {
    let function_name = call_stack.last().unwrap().function_name();
    let syscall_resources = versioned_constants
        .os_resources
        .execute_syscalls
        .get(
            &function_name
                .0
                .parse::<DeprecatedSyscallSelector>()
                .expect("Failed to map function to SyscallSelector"),
        )
        .unwrap();

    let measurements = if sierra_gas_tracking {
        calculate_syscall_sierra_gas_measurements(
            syscall_resources,
            invocations,
            versioned_constants,
        )
    } else {
        calculate_syscall_cairo_steps_measurements(syscall_resources, invocations)
    };

    Sample {
        call_stack,
        measurements,
    }
}

fn calculate_syscall_sierra_gas_measurements(
    resources: &VmExecutionResources,
    invocations: i64,
    versioned_constants: &VersionedConstants,
) -> HashMap<MeasurementUnit, MeasurementValue> {
    let step_cost = usize::try_from(versioned_constants.os_constants.step_gas_cost)
        .expect("Overflow while converting step_gas_cost to usize");
    let memory_hole_cost = usize::try_from(versioned_constants.os_constants.memory_hole_gas_cost)
        .expect("Overflow while converting memory_hole_gas_cost to usize");

    let from_steps = resources
        .n_steps
        .checked_mul(step_cost)
        .expect("Overflow while calculating sierra gas from steps");
    let from_memory_holes = resources
        .n_memory_holes
        .checked_mul(memory_hole_cost)
        .expect("Overflow while calculating sierra gas from memory_holes");
    let from_builtins: usize = resources
        .builtin_instance_counter
        .iter()
        .map(|(builtin, &amount)| {
            usize::try_from(get_builtin_gas_cost(
                builtin,
                &versioned_constants.os_constants.builtin_gas_costs,
            ))
            .expect("Overflow while converting builtin_gas_cost to usize")
            .checked_add(amount)
            .expect("Overflow while calculating sierra gas from builtins")
        })
        .sum();

    let total_sierra_cost = from_steps + from_memory_holes + from_builtins;
    let syscall_base_gas_cost = usize::try_from(
        versioned_constants
            .os_constants
            .syscall_base_gas_cost
            .step_gas_cost,
    )
    .expect("Overflow while converting syscall_base_gas_cost to usize");
    // syscalls have minimal sierra cost, we need to make sure it is being respected
    let real_syscall_cost = std::cmp::max(
        total_sierra_cost,
        syscall_base_gas_cost
            .checked_mul(step_cost)
            .expect("Overflow while calculating minimal syscall cost"),
    );
    let total_cost = i64::try_from(real_syscall_cost)
        .expect("Overflow while converting syscall cost to i64")
        .checked_mul(invocations)
        .expect("Total syscall cost multiplication overflow");

    vec![(
        MeasurementUnit::from("sierra_gas".to_string()),
        MeasurementValue(total_cost),
    )]
    .into_iter()
    .collect()
}

fn calculate_syscall_cairo_steps_measurements(
    resources: &VmExecutionResources,
    invocations: i64,
) -> HashMap<MeasurementUnit, MeasurementValue> {
    let multiply_resource_by_invocations = |resource: usize, invocations: i64| -> i64 {
        let resource = i64::try_from(resource).expect("Overflow while converting resource to i64");
        resource
            .checked_mul(invocations)
            .expect("Measurement multiplication overflow")
    };

    let mut measurements: HashMap<MeasurementUnit, MeasurementValue> = vec![
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
        measurements.insert(
            MeasurementUnit::from(builtin.to_string()),
            MeasurementValue(multiply_resource_by_invocations(*b_count, invocations)),
        );
    }
    measurements
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
