use core::fmt;
use std::collections::HashMap;
use std::fmt::Display;

use trace_data::{ContractAddress, EntryPointSelector};

use trace_data::{CallTrace, ExecutionResources};

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct FunctionName(pub String);

impl FunctionName {
    #[inline]
    fn from(entry_point_id: &EntryPointId) -> FunctionName {
        FunctionName(format!("{entry_point_id}"))
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct MeasurementUnit(pub String);
impl MeasurementUnit {
    fn from(name: &str) -> Self {
        MeasurementUnit(String::from(name))
    }
}

#[derive(Debug, Clone)]
pub struct MeasurementValue(pub i64);

#[allow(clippy::struct_field_names)]
pub struct ContractCallSample {
    pub call_stack: Vec<FunctionName>,
    pub measurements: HashMap<MeasurementUnit, MeasurementValue>,
}

impl ContractCallSample {
    pub fn from(call_stack: Vec<FunctionName>, resources: ExecutionResources) -> Self {
        let mut measurements: HashMap<MeasurementUnit, MeasurementValue> = vec![
            (MeasurementUnit::from("calls"), MeasurementValue(1)),
            (
                MeasurementUnit::from("steps"),
                MeasurementValue(i64::try_from(resources.vm_resources.n_steps).unwrap()),
            ),
            (
                MeasurementUnit::from("memory_holes"),
                MeasurementValue(i64::try_from(resources.vm_resources.n_memory_holes).unwrap()),
            ),
        ]
        .into_iter()
        .collect();

        // TODO

        for (builtin, count) in &resources.vm_resources.builtin_instance_counter {
            assert!(measurements.get(&MeasurementUnit::from(builtin)).is_none());
            measurements.insert(
                MeasurementUnit::from(builtin),
                MeasurementValue(i64::try_from(*count).unwrap()),
            );
        }

        let syscall_counter_with_string: Vec<_> = resources
            .syscall_counter
            .iter()
            .map(|(syscall, count)| (format!("{syscall:?}"), *count))
            .collect();
        for (syscall, count) in &syscall_counter_with_string {
            assert!(measurements.get(&MeasurementUnit::from(syscall)).is_none());
            measurements.insert(
                MeasurementUnit::from(syscall),
                MeasurementValue(i64::try_from(*count).unwrap()),
            );
        }

        ContractCallSample {
            call_stack,
            measurements,
        }
    }
}

pub struct EntryPointId {
    address: String,
    selector: String,
    contract_name: Option<String>,
    function_name: Option<String>,
}

impl EntryPointId {
    fn from(
        contract_name: Option<String>,
        function_name: Option<String>,
        contract_address: ContractAddress,
        selector: EntryPointSelector,
    ) -> Self {
        EntryPointId {
            address: contract_address.0,
            selector: selector.0,
            contract_name,
            function_name,
        }
    }
}

impl Display for EntryPointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contract_name = self
            .contract_name
            .clone()
            .unwrap_or(String::from("<unknown>"));
        let function_name = self
            .function_name
            .clone()
            .unwrap_or(String::from("<unknown>"));
        write!(
            f,
            "Contract address: {}\n Selector: {}\nContract name: {}\nFunction name: {}\n",
            self.address, self.selector, contract_name, function_name
        )
    }
}

pub fn collect_samples_from_trace(trace: &CallTrace) -> Vec<ContractCallSample> {
    let mut samples = vec![];
    let mut current_path = vec![];
    collect_samples(&mut samples, &mut current_path, trace);
    samples
}

fn collect_samples<'a>(
    samples: &mut Vec<ContractCallSample>,
    current_call_stack: &mut Vec<EntryPointId>,
    trace: &'a CallTrace,
) -> &'a ExecutionResources {
    current_call_stack.push(EntryPointId::from(
        trace.entry_point.contract_name.clone(),
        trace.entry_point.function_name.clone(),
        trace.entry_point.contract_address.clone(),
        trace.entry_point.entry_point_selector.clone(),
    ));

    let mut children_resources = ExecutionResources::default();
    for sub_trace in &trace.nested_calls {
        children_resources += &collect_samples(samples, current_call_stack, sub_trace);
    }

    samples.push(ContractCallSample::from(
        current_call_stack.iter().map(FunctionName::from).collect(),
        &trace.cumulative_resources - &children_resources,
    ));

    current_call_stack.pop();

    &trace.cumulative_resources
}
