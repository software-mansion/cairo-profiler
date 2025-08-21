use cairo_annotations::annotations::profiler::FunctionName;
use cairo_annotations::trace_data::{ExecutionResources, L1Resources};
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::ops::Add;

#[derive(Clone, Debug)]
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
    Libfunc(FunctionName),
}

impl InternalFunctionCall {
    pub fn function_name(&self) -> &FunctionName {
        match self {
            InternalFunctionCall::Inlined(function_name)
            | InternalFunctionCall::NonInlined(function_name)
            | InternalFunctionCall::Syscall(function_name)
            | InternalFunctionCall::Libfunc(function_name) => function_name,
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

impl PartialEq<i64> for MeasurementValue {
    fn eq(&self, other: &i64) -> bool {
        self.0 == *other
    }
}

impl Add for MeasurementValue {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        MeasurementValue(self.0 + other.0)
    }
}

impl Sample {
    pub fn from(
        call_stack: Vec<FunctionCall>,
        resources: &ExecutionResources,
        l1_resources: &L1Resources,
        l2_gas: Option<usize>,
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
            (
                MeasurementUnit::from("sierra_gas".to_string()),
                MeasurementValue(i64::try_from(resources.gas_consumed.unwrap_or(0)).unwrap()),
            ),
            (
                MeasurementUnit::from("l2_gas".to_string()),
                MeasurementValue(i64::try_from(l2_gas.unwrap_or(0)).unwrap()),
            ),
        ]
        .into_iter()
        .filter(|(_, value)| *value != 0)
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

        l1_resources
            .l2_l1_message_sizes
            .iter()
            .sum::<usize>()
            .try_into()
            .ok()
            .filter(|summarized_resources| *summarized_resources > 0)
            .map(|summarized_resources| {
                measurements.insert(
                    MeasurementUnit::from("l2_l1_message_sizes".to_string()),
                    MeasurementValue(summarized_resources),
                )
            });

        Sample {
            call_stack,
            measurements,
        }
    }
}
