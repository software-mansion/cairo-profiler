use core::fmt;
use std::collections::HashMap;
use std::fmt::Display;

use trace_data::{ContractAddress, EntryPointSelector};

use trace_data::{CallTrace, ExecutionResources, L1Resources};

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

pub struct ContractCallSample {
    pub call_stack: Vec<FunctionName>,
    pub measurements: HashMap<MeasurementUnit, MeasurementValue>,
}

impl ContractCallSample {
    pub fn from(
        call_stack: Vec<FunctionName>,
        resources: &ExecutionResources,
        l1_resources: &L1Resources,
    ) -> Self {
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

        for (builtin, count) in &resources.vm_resources.builtin_instance_counter {
            assert!(!measurements.contains_key(&MeasurementUnit::from(builtin)));
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
            assert!(!measurements.contains_key(&MeasurementUnit::from(syscall)));
            measurements.insert(
                MeasurementUnit::from(syscall),
                MeasurementValue(i64::try_from(*count).unwrap()),
            );
        }

        assert!(!measurements.contains_key(&MeasurementUnit::from("l2_l1_message_sizes")));
        let summarized_payload: usize = l1_resources.l2_l1_message_sizes.iter().sum();
        measurements.insert(
            MeasurementUnit::from("l2_l1_message_sizes"),
            MeasurementValue(i64::try_from(summarized_payload).unwrap()),
        );

        assert!(!measurements.contains_key(&MeasurementUnit::from("storage_values_updated")));
        let summarized_payload = l1_resources.storage_values_updated;
        measurements.insert(
            MeasurementUnit::from("storage_values_updated"),
            MeasurementValue(i64::try_from(summarized_payload).unwrap()),
        );

        ContractCallSample {
            call_stack,
            measurements,
        }
    }
}

/// `contract_name` and `function_name` are always present (in case they are not in trace we just
/// set `<unknown>` string)
/// `address` and `selector` are optional and set if `--show-details` flag is enabled
/// or names are unknown
pub struct EntryPointId {
    address: Option<String>,
    selector: Option<String>,
    contract_name: String,
    function_name: String,
}

impl EntryPointId {
    fn from(
        contract_name: Option<String>,
        function_name: Option<String>,
        contract_address: ContractAddress,
        function_selector: EntryPointSelector,
        show_details: bool,
    ) -> Self {
        let (contract_name, address) = match contract_name {
            Some(name) if show_details => (name, Some(contract_address.0)),
            Some(name) => (name, None),
            None => (String::from("<unknown>"), Some(contract_address.0)),
        };

        let (function_name, selector) = match function_name {
            Some(name) if show_details => (name, Some(function_selector.0)),
            Some(name) => (name, None),
            None => (String::from("<unknown>"), Some(function_selector.0)),
        };

        EntryPointId {
            address,
            selector,
            contract_name,
            function_name,
        }
    }
}

impl Display for EntryPointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contract_address = match &self.address {
            None => String::new(),
            Some(address) => format!("Address: {address}\n"),
        };
        let selector = match &self.selector {
            None => String::new(),
            Some(selector) => format!("Selector: {selector}\n"),
        };

        write!(
            f,
            "Contract: {}\n{contract_address}Function: {}\n{selector}",
            self.contract_name, self.function_name
        )
    }
}

pub fn collect_samples_from_trace(
    trace: &CallTrace,
    show_details: bool,
) -> Vec<ContractCallSample> {
    let mut samples = vec![];
    let mut current_path = vec![];
    collect_samples(&mut samples, &mut current_path, trace, show_details);
    samples
}

fn collect_samples<'a>(
    samples: &mut Vec<ContractCallSample>,
    current_call_stack: &mut Vec<EntryPointId>,
    trace: &'a CallTrace,
    show_details: bool,
) -> (&'a ExecutionResources, isize) {
    current_call_stack.push(EntryPointId::from(
        trace.entry_point.contract_name.clone(),
        trace.entry_point.function_name.clone(),
        trace.entry_point.contract_address.clone(),
        trace.entry_point.entry_point_selector.clone(),
        show_details,
    ));

    let mut children_resources = ExecutionResources::default();
    let mut children_storage_updates = 0;
    for sub_trace in &trace.nested_calls {
        let (child_resources, child_storage_updates) =
            &collect_samples(samples, current_call_stack, sub_trace, show_details);
        children_resources += child_resources;
        children_storage_updates += child_storage_updates;
    }

    samples.push(ContractCallSample::from(
        current_call_stack.iter().map(FunctionName::from).collect(),
        &(&trace.cumulative_resources - &children_resources),
        &L1Resources {
            l2_l1_message_sizes: trace.used_l1_resources.l2_l1_message_sizes.clone(),
            storage_values_updated: trace.used_l1_resources.storage_values_updated
                - children_storage_updates,
        },
    ));

    current_call_stack.pop();

    (
        &trace.cumulative_resources,
        trace.used_l1_resources.storage_values_updated,
    )
}
