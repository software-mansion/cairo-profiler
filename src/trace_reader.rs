use core::fmt;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::ops::Add;

use crate::profile_builder::perftools::profiles::ValueType;
use crate::profile_builder::{ProfilerContext, StringId};
use crate::trace_data::{ContractAddress, EntryPointSelector};

use crate::trace_data::{CallTrace, DeprecatedSyscallSelector, ExecutionResources, OnchainData};

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct FunctionName(pub String);

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Location(pub Vec<FunctionName>);

impl Location {
    #[inline]
    fn from(s: &[EntryPointId]) -> Location {
        Location(s.iter().map(|c| FunctionName(format!("{c}"))).collect())
    }
}

pub enum SampleType {
    ContractCall,
}

#[allow(clippy::struct_field_names)]
pub struct Sample {
    pub location: Location,
    pub sample_type: SampleType,
    pub flat_resources: ExecutionResources,
    pub onchain_data: OnchainData,
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

        assert!(measurements_map.get("l2_l1_message_sizes").is_none());
        let summarized_payload: usize = self.onchain_data.l2_l1_message_sizes.iter().sum();
        measurements_map.insert(
            "l2_l1_message_sizes",
            i64::try_from(summarized_payload).unwrap(),
        );

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
    pub builtins: HashSet<String>,
    pub syscalls: HashSet<DeprecatedSyscallSelector>,
    pub onchain_data: HashSet<(String, String)>,
}

impl ResourcesKeys {
    pub fn measurement_types(&self, context: &mut ProfilerContext) -> Vec<ValueType> {
        let mut value_types = vec![];

        for builtin in &self.builtins {
            let unit_string = " ".to_string().add(&builtin.replace('_', " "));
            value_types.push(ValueType {
                r#type: context.string_id(builtin).into(),
                unit: context.string_id(&unit_string).into(),
            });
        }
        for syscall in &self.syscalls {
            let type_string = format!("{syscall:?}");
            let unit_string = " ".to_string().add(&type_string);

            value_types.push(ValueType {
                r#type: context.string_id(&type_string).into(),
                unit: context.string_id(&unit_string).into(),
            });
        }
        for (type_name, unit_name) in &self.onchain_data {
            value_types.push(ValueType {
                r#type: context.string_id(type_name).into(),
                unit: context.string_id(unit_name).into(),
            });
        }

        value_types
    }
}

pub fn collect_resources_keys(samples: &[Sample]) -> ResourcesKeys {
    let mut syscalls = HashSet::new();
    let mut builtins = HashSet::new();
    let mut onchain_data = HashSet::new();
    for sample in samples {
        builtins.extend(
            sample
                .flat_resources
                .vm_resources
                .builtin_instance_counter
                .keys()
                .cloned(),
        );
        syscalls.extend(sample.flat_resources.syscall_counter.keys());

        if !sample.onchain_data.l2_l1_message_sizes.is_empty() {
            onchain_data.insert((
                "l2_l1_message_sizes".to_string(),
                " payload length".to_string(),
            ));
        }
    }

    ResourcesKeys {
        builtins,
        syscalls,
        onchain_data,
    }
}

fn collect_samples<'a>(
    samples: &mut Vec<Sample>,
    current_path: &mut Vec<EntryPointId>,
    trace: &'a CallTrace,
) -> &'a ExecutionResources {
    current_path.push(EntryPointId::from(
        trace.entry_point.contract_name.clone(),
        trace.entry_point.function_name.clone(),
        trace.entry_point.contract_address.clone(),
        trace.entry_point.entry_point_selector.clone(),
    ));

    let mut children_resources = ExecutionResources::default();
    for sub_trace in &trace.nested_calls {
        children_resources += &collect_samples(samples, current_path, sub_trace);
    }

    samples.push(Sample {
        location: Location::from(current_path),
        sample_type: SampleType::ContractCall,
        flat_resources: &trace.cumulative_resources - &children_resources,
        onchain_data: trace.used_onchain_data.clone(),
    });

    current_path.pop();

    &trace.cumulative_resources
}
