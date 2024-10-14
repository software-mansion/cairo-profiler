use crate::trace_reader::function_name::FunctionName;
use crate::trace_reader::sample::InternalFunctionCall::Syscall;
use crate::trace_reader::sample::{FunctionCall, MeasurementUnit, MeasurementValue, Sample};
use anyhow::Result;
use camino::Utf8PathBuf;
use serde_json::{from_str, from_value, Value};
use std::fs;
use trace_data::{DeprecatedSyscallSelector, OsResources};

pub fn collect_syscall_sample(
    mut call_stack: Vec<FunctionCall>,
    syscall: DeprecatedSyscallSelector,
    count: usize,
    os_resources_map: &OsResources,
) -> Sample {
    call_stack.push(FunctionCall::InternalFunctionCall(Syscall(FunctionName(
        format!("syscall: {syscall:?}"),
    ))));
    let resources = os_resources_map
        .execute_syscalls
        .get(&syscall)
        .unwrap_or_else(|| panic!("Missing syscall {syscall:?} from versioned constants file"));
    Sample {
        call_stack,
        measurements: {
            let mut measurements = vec![
                (
                    MeasurementUnit::from("calls".to_string()),
                    MeasurementValue(
                        (count)
                            .try_into()
                            .expect("Overflow while converting to i64"),
                    ),
                ),
                (
                    MeasurementUnit::from("steps".to_string()),
                    MeasurementValue(
                        resources
                            .n_steps
                            .checked_mul(count)
                            .expect("Multiplication overflow")
                            .try_into()
                            .expect("Overflow while converting to i64"),
                    ),
                ),
                (
                    MeasurementUnit::from("memory_holes".to_string()),
                    MeasurementValue(
                        resources
                            .n_memory_holes
                            .checked_mul(count)
                            .expect("Multiplication overflow")
                            .try_into()
                            .expect("Overflow while converting to i64"),
                    ),
                ),
            ];

            for (builtin, b_count) in &resources.builtin_instance_counter {
                measurements.push((
                    MeasurementUnit::from(builtin.to_string()),
                    MeasurementValue(
                        b_count
                            .checked_mul(count)
                            .expect("Multiplication overflow")
                            .try_into()
                            .expect("Overflow while converting to i64"),
                    ),
                ));
            }

            measurements.into_iter().collect()
        },
    }
}

pub fn read_and_parse_versioned_constants_file(path: &Option<Utf8PathBuf>) -> Result<OsResources> {
    let file_content = match path {
        Some(path) => fs::read_to_string(path)?,
        // include_str requires a string literal
        None => include_str!("../../resources/versioned_constants_0_13_2_1.json").to_string(),
    };
    let json_value: Value = from_str(&file_content)?;
    let parsed_resources = json_value
        .get("os_resources")
        .expect("Field 'os_resources' not found in versioned constants file");
    let os_resources: OsResources = from_value(parsed_resources.clone())?;
    Ok(os_resources)
}
