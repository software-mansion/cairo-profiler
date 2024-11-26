use cairo_annotations::annotations::profiler::FunctionName;
use cairo_annotations::trace_data::{ExecutionResources, L1Resources};
use std::collections::HashMap;

pub(crate) struct Sample {
    pub call_stack: Vec<FunctionCall>,
    pub measurements: HashMap<MeasurementUnit, MeasurementValue>,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum FunctionCall {
    EntrypointCall(FunctionName),
    InternalFunctionCall(InternalFunctionCall),
}

impl FunctionCall {
    pub fn function_name(&self) -> &FunctionName {
        match self {
            FunctionCall::EntrypointCall(function_name) => function_name,
            FunctionCall::InternalFunctionCall(internal_function_call) => {
                internal_function_call.function_name()
            }
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum InternalFunctionCall {
    Inlined(FunctionName),
    NonInlined(FunctionName),
    Syscall(FunctionName),
}

impl InternalFunctionCall {
    pub fn function_name(&self) -> &FunctionName {
        match self {
            InternalFunctionCall::Inlined(function_name)
            | InternalFunctionCall::NonInlined(function_name)
            | InternalFunctionCall::Syscall(function_name) => function_name,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct MeasurementUnit(pub String);

impl From<String> for MeasurementUnit {
    fn from(value: String) -> Self {
        MeasurementUnit(value)
    }
}

#[derive(Debug, Clone)]
pub struct MeasurementValue(pub i64);

impl Sample {
    pub fn from(
        call_stack: Vec<FunctionCall>,
        resources: &ExecutionResources,
        l1_resources: &L1Resources,
    ) -> Self {
        let mut measurements: HashMap<MeasurementUnit, MeasurementValue> = vec![
            (
                MeasurementUnit::from("calls".to_string()),
                MeasurementValue(1),
            ),
            (
                MeasurementUnit::from("steps".to_string()),
                MeasurementValue(i64::try_from(resources.vm_resources.n_steps).unwrap()),
            ),
            (
                MeasurementUnit::from("memory_holes".to_string()),
                MeasurementValue(i64::try_from(resources.vm_resources.n_memory_holes).unwrap()),
            ),
        ]
        .into_iter()
        .collect();

        for (builtin, count) in &resources.vm_resources.builtin_instance_counter {
            assert!(!measurements.contains_key(&MeasurementUnit::from(builtin.to_string())));
            measurements.insert(
                MeasurementUnit::from(builtin.to_string()),
                MeasurementValue(i64::try_from(*count).unwrap()),
            );
        }

        assert!(
            !measurements.contains_key(&MeasurementUnit::from("l2_l1_message_sizes".to_string()))
        );
        let summarized_payload: usize = l1_resources.l2_l1_message_sizes.iter().sum();
        measurements.insert(
            MeasurementUnit::from("l2_l1_message_sizes".to_string()),
            MeasurementValue(i64::try_from(summarized_payload).unwrap()),
        );

        Sample {
            call_stack,
            measurements,
        }
    }
}
