use crate::trace_reader::function_name::FunctionName;
use std::collections::HashMap;
use trace_data::{ExecutionResources, L1Resources};

// In pprof inlining is signalised by locations having multiple lines.
// A sample with stack [A, B_inlined, C, D, E_inlined, F_inlined] has 3 pprof locations
// (instead of 6 if all function calls were not inlined) corresponding to the following stacks:
// [A, B_inlined], [C] and [D, E_inlined, F_inlined].
pub struct AggregatedSample {
    pub aggregated_call_stack: Vec<Vec<FunctionCall>>,
    pub measurements: HashMap<MeasurementUnit, MeasurementValue>,
}

impl From<Sample> for AggregatedSample {
    fn from(sample: Sample) -> Self {
        // This vector represent stacks of functions corresponding to single locations.
        // It contains tuples of form (start_index, end_index).
        // A single stack is `&call_stack[start_index..=end_index]`.
        let mut function_stacks_indexes = vec![];

        let mut current_function_stack_start_index = 0;
        for (index, function_call) in sample.call_stack.iter().enumerate() {
            match function_call {
                FunctionCall::InternalFunctionCall(InternalFunction::NonInlined(_))
                | FunctionCall::EntrypointCall(_) => {
                    if index != 0 {
                        function_stacks_indexes
                            .push((current_function_stack_start_index, index - 1));
                    }
                    current_function_stack_start_index = index;
                }
                FunctionCall::InternalFunctionCall(InternalFunction::Inlined(_)) => {}
            }
        }

        function_stacks_indexes.push((
            current_function_stack_start_index,
            sample.call_stack.len() - 1,
        ));

        let mut aggregated_call_stack = vec![];
        let call_stack_iter = sample.call_stack.into_iter();
        for (start_index, end_index) in function_stacks_indexes {
            aggregated_call_stack.push(
                call_stack_iter
                    .clone()
                    .take(end_index - start_index + 1)
                    .collect(),
            );
        }

        AggregatedSample {
            aggregated_call_stack,
            measurements: sample.measurements,
        }
    }
}

pub(super) struct Sample {
    pub call_stack: Vec<FunctionCall>,
    pub measurements: HashMap<MeasurementUnit, MeasurementValue>,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum FunctionCall {
    EntrypointCall(FunctionName),
    InternalFunctionCall(InternalFunction),
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum InternalFunction {
    #[allow(dead_code)]
    Inlined(FunctionName),
    NonInlined(FunctionName),
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

        let syscall_counter_with_string: Vec<_> = resources
            .syscall_counter
            .iter()
            .map(|(syscall, count)| (format!("{syscall:?}"), *count))
            .collect();
        for (syscall, count) in &syscall_counter_with_string {
            assert!(!measurements.contains_key(&MeasurementUnit::from(syscall.to_string())));
            measurements.insert(
                MeasurementUnit::from(syscall.to_string()),
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
