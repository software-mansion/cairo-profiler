use crate::trace_reader::function_trace_builder::Steps;
use crate::trace_reader::sample::{
    FunctionCall, InternalFunctionCall, MeasurementUnit, MeasurementValue, Sample,
};
use crate::versioned_constants_reader::OsResources;
use anyhow::{anyhow, Result};
use itertools::Itertools;
use std::collections::HashMap;
use trace_data::DeprecatedSyscallSelector;

pub(crate) fn stack_trace_to_samples(
    functions_stack_traces: HashMap<Vec<FunctionCall>, Steps>,
    os_resources_map: &OsResources,
) -> Vec<Sample> {
    functions_stack_traces
        .into_iter()
        .map(|(call_stack, steps)| {
            let mut measurements: HashMap<MeasurementUnit, MeasurementValue> = vec![(
                MeasurementUnit::from("steps".to_string()),
                MeasurementValue(i64::try_from(steps.0).unwrap()),
            )]
            .into_iter()
            .collect();

            if let Some(FunctionCall::InternalFunctionCall(InternalFunctionCall::Syscall(
                function_name,
            ))) = call_stack.last()
            {
                let syscall = map_syscall_name_to_selector(function_name.0.as_str())
                    .expect("Failed to map syscall");
                let resources = os_resources_map.execute_syscalls.get(&syscall).unwrap();

                if let Some(value) =
                    measurements.get_mut(&MeasurementUnit::from("steps".to_string()))
                {
                    *value += MeasurementValue(i64::try_from(resources.n_steps).unwrap());
                }

                measurements.insert(
                    MeasurementUnit::from("memory_holes".to_string()),
                    MeasurementValue(i64::try_from(resources.n_memory_holes).unwrap()),
                );

                for (builtin, b_count) in &resources.builtin_instance_counter {
                    measurements.insert(
                        MeasurementUnit::from(builtin.to_string()),
                        MeasurementValue(i64::try_from(*b_count).unwrap()),
                    );
                }
            }

            Sample {
                call_stack,
                measurements,
            }
        })
        .collect_vec()
}

fn map_syscall_name_to_selector(syscall: &str) -> Result<DeprecatedSyscallSelector> {
    match syscall {
        "call_contract_syscall" => Ok(DeprecatedSyscallSelector::CallContract),
        "deploy_syscall" => Ok(DeprecatedSyscallSelector::Deploy),
        "emit_event_syscall" => Ok(DeprecatedSyscallSelector::EmitEvent),
        "get_block_hash_syscall" => Ok(DeprecatedSyscallSelector::GetBlockHash),
        "get_execution_info_syscall" | "get_execution_info_v2_syscall" => {
            Ok(DeprecatedSyscallSelector::GetExecutionInfo)
        }
        "keccak_syscall" => Ok(DeprecatedSyscallSelector::Keccak),
        "library_call_syscall" => Ok(DeprecatedSyscallSelector::LibraryCall),
        "replace_class_syscall" => Ok(DeprecatedSyscallSelector::ReplaceClass),
        "send_message_to_l1_syscall" => Ok(DeprecatedSyscallSelector::SendMessageToL1),
        "storage_read_syscall" => Ok(DeprecatedSyscallSelector::StorageRead),
        "storage_write_syscall" => Ok(DeprecatedSyscallSelector::StorageWrite),
        "sha256_process_block_syscall" => Ok(DeprecatedSyscallSelector::Sha256ProcessBlock),
        _ => Err(anyhow!("Missing mapping for {syscall:?}")),
    }
}
