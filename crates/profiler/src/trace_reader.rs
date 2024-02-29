use core::fmt;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::ops::Add;

use crate::profile_builder::perftools::profiles::ValueType;
use crate::profile_builder::{ProfilerContext, StringId};
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

pub enum SampleType {
    ContractCall,
}

#[allow(clippy::struct_field_names)]
pub struct Sample {
    pub call_stack: Vec<FunctionName>,
    pub sample_type: SampleType,
    pub flat_resources: ExecutionResources,
}

impl Sample {
    pub fn extract_measurements(
        &self,
        measurement_types: &[ValueType],
        context: &ProfilerContext,
    ) -> Vec<i64> {
        let mut measurements_map: HashMap<&str, i64> = vec![
            ("calls", 1),
            (
                "n_steps",
                i64::try_from(self.flat_resources.vm_resources.n_steps).unwrap(),
            ),
            (
                "n_memory_holes",
                i64::try_from(self.flat_resources.vm_resources.n_memory_holes).unwrap(),
            ),
        ]
        .into_iter()
        .collect();

        for (builtin, count) in &self.flat_resources.vm_resources.builtin_instance_counter {
            assert!(measurements_map.get(&&**builtin).is_none());
            measurements_map.insert(builtin, i64::try_from(*count).unwrap());
        }

        let syscall_counter_with_string: Vec<_> = self
            .flat_resources
            .syscall_counter
            .iter()
            .map(|(syscall, count)| (format!("{syscall:?}"), *count))
            .collect();
        for (syscall, count) in &syscall_counter_with_string {
            assert!(measurements_map.get(&&**syscall).is_none());
            measurements_map.insert(syscall, i64::try_from(*count).unwrap());
        }

        let mut measurements = vec![];
        for value_type in measurement_types {
            let value_type_str =
                context.string_from_string_id(StringId(u64::try_from(value_type.r#type).unwrap()));
            measurements.push(*measurements_map.get(value_type_str).unwrap_or(&0));
        }

        measurements
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

pub fn collect_samples_from_trace(trace: &CallTrace) -> Vec<Sample> {
    let mut samples = vec![];
    let mut current_path = vec![];
    collect_samples(&mut samples, &mut current_path, trace);
    samples
}

pub struct ResourcesKeys {
    pub keys: HashSet<String>,
}

impl ResourcesKeys {
    pub fn measurement_types(&self, context: &mut ProfilerContext) -> Vec<ValueType> {
        let mut value_types = vec![];

        for key in &self.keys {
            let unit_string = " ".to_string().add(&key.replace('_', " "));
            value_types.push(ValueType {
                r#type: context.string_id(key).into(),
                unit: context.string_id(&unit_string).into(),
            });
        }

        value_types
    }
}

pub fn collect_resources_keys(samples: &[Sample]) -> ResourcesKeys {
    let mut keys = HashSet::new();
    for sample in samples {
        keys.extend(
            sample
                .flat_resources
                .vm_resources
                .builtin_instance_counter
                .keys()
                .cloned(),
        );
        keys.extend(
            sample
                .flat_resources
                .syscall_counter
                .keys()
                .map(|x| format!("{x:?}")),
        );
    }
    ResourcesKeys { keys }
}

fn collect_samples<'a>(
    samples: &mut Vec<Sample>,
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

    samples.push(Sample {
        call_stack: current_call_stack.iter().map(FunctionName::from).collect(),
        sample_type: SampleType::ContractCall,
        flat_resources: &trace.cumulative_resources - &children_resources,
    });

    current_call_stack.pop();

    &trace.cumulative_resources
}
