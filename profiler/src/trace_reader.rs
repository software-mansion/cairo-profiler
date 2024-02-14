use core::fmt;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;

use crate::profile_builder::perftools::profiles::ValueType;
use crate::profile_builder::{ProfilerContext, StringId};
use starknet_api::core::{ContractAddress, EntryPointSelector};

use crate::trace_data::{CallTrace, DeprecatedSyscallSelector, ExecutionResources};

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct FunctionName(pub String);

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Location(pub Vec<FunctionName>);

impl Location {
    #[inline]
    fn from(s: &[EntryPointId]) -> Location {
        Location(s.iter().map(|c| FunctionName(format!("{}", c))).collect())
    }
}

pub enum SampleType {
    ContractCall,
}

pub struct Sample {
    pub location: Location,
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
            ("n_steps", self.flat_resources.vm_resources.n_steps as i64),
            (
                "n_memory_holes",
                self.flat_resources.vm_resources.n_memory_holes as i64,
            ),
        ]
        .into_iter()
        .collect();

        for (builtin, count) in &self.flat_resources.vm_resources.builtin_instance_counter {
            assert!(measurements_map.get(&&**builtin).is_none());
            measurements_map.insert(builtin, *count as i64);
        }

        let syscall_counter_with_string: Vec<_> = self
            .flat_resources
            .syscall_counter
            .iter()
            .map(|(syscall, count)| (format!("{syscall:?}"), *count))
            .collect();
        for (syscall, count) in &syscall_counter_with_string {
            assert!(measurements_map.get(&&**syscall).is_none());
            measurements_map.insert(syscall, *count as i64);
        }

        let mut measurements = vec![];
        for value_type in measurement_types {
            let value_type_str = context.string_from_string_id(StringId(value_type.r#type as u64));
            measurements.push(*measurements_map.get(value_type_str).unwrap_or(&0))
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
        let address_str = format!("0x{}", hex::encode(contract_address.0.key().bytes()));
        let selector_str = format!("{}", selector.0);
        EntryPointId {
            address: address_str,
            selector: selector_str,
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
    pub builtins: HashSet<String>,
    pub syscalls: HashSet<DeprecatedSyscallSelector>,
}

impl ResourcesKeys {
    pub fn measurement_types(&self, context: &mut ProfilerContext) -> Vec<ValueType> {
        let mut value_types = vec![];

        for builtin in &self.builtins {
            value_types.push(ValueType {
                r#type: context.string_id(builtin).into(),
                unit: context.string_id(&String::from("count")).into(),
            });
        }
        for syscall in &self.syscalls {
            value_types.push(ValueType {
                r#type: context.string_id(&format!("{syscall:?}")).into(),
                unit: context.string_id(&String::from("count")).into(),
            });
        }

        value_types
    }
}

pub fn collect_resources_keys(samples: &[Sample]) -> ResourcesKeys {
    let mut syscalls = HashSet::new();
    let mut builtins = HashSet::new();
    for sample in samples {
        builtins.extend(
            sample
                .flat_resources
                .vm_resources
                .builtin_instance_counter
                .keys()
                .cloned(),
        );
        syscalls.extend(sample.flat_resources.syscall_counter.keys())
    }
    ResourcesKeys { syscalls, builtins }
}

fn collect_samples<'a>(
    samples: &mut Vec<Sample>,
    current_path: &mut Vec<EntryPointId>,
    trace: &'a CallTrace,
) -> &'a ExecutionResources {
    current_path.push(EntryPointId::from(
        trace.entry_point.contract_name.clone(),
        trace.entry_point.function_name.clone(),
        trace.entry_point.storage_address,
        trace.entry_point.entry_point_selector,
    ));

    let mut children_resources = ExecutionResources::default();
    for sub_trace in &trace.nested_calls {
        children_resources += &collect_samples(samples, current_path, sub_trace);
    }

    samples.push(Sample {
        location: Location::from(current_path),
        sample_type: SampleType::ContractCall,
        flat_resources: &trace.cumulative_resources - &children_resources,
    });

    current_path.pop();

    &trace.cumulative_resources
}
