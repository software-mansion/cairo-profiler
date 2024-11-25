use crate::trace_reader::function_trace_builder::Steps;
use crate::trace_reader::sample::{FunctionCall, MeasurementUnit, MeasurementValue, Sample};
use crate::versioned_constants_reader::OsResources;
use cairo_annotations::trace_data::DeprecatedSyscallSelector;
use cairo_lang_sierra::extensions::starknet::StarkNetConcreteLibfunc;
use itertools::Itertools;
use std::collections::HashMap;

pub fn trace_to_samples(
    functions_stack_traces: HashMap<Vec<FunctionCall>, Steps>,
    syscall_stack_traces: HashMap<Vec<FunctionCall>, i64>,
    os_resources_map: &OsResources,
) -> Vec<Sample> {
    let multiply_resource_by_invocations = |resource: usize, invocations: i64| -> i64 {
        let resource = i64::try_from(resource).expect("Overflow while converting resource to i64");
        resource
            .checked_mul(invocations)
            .expect("Multiplication overflow")
    };

    let mut function_samples = functions_stack_traces
        .into_iter()
        .map(|(call_stack, steps)| {
            let measurements: HashMap<MeasurementUnit, MeasurementValue> = vec![(
                MeasurementUnit::from("steps".to_string()),
                MeasurementValue(i64::try_from(steps.0).unwrap()),
            )]
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
            let resources = os_resources_map
                .execute_syscalls
                .get(
                    &function_name
                        .0
                        .as_str()
                        .parse()
                        .expect("Unknown syscall found"),
                )
                .expect("Resource map is expected to contain all syscalls");
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
        _ => panic!("Missing mapping to a syscall"),
    }
}
