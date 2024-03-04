use core::fmt;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::ops::Add;

use crate::profile_builder::perftools::profiles::ValueType;
use crate::profile_builder::{ProfilerContext, StringId};
use trace_data::{ContractAddress, EntryPointSelector};

use trace_data::{CallTrace, DeprecatedSyscallSelector, ExecutionResources};

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

pub fn collect_samples_from_trace(trace: &CallTrace, show_details: bool) -> Vec<Sample> {
    let mut samples = vec![];
    let mut current_path = vec![];
    collect_samples(&mut samples, &mut current_path, trace, show_details);
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
        syscalls.extend(sample.flat_resources.syscall_counter.keys());
    }
    ResourcesKeys { builtins, syscalls }
}

fn collect_samples<'a>(
    samples: &mut Vec<Sample>,
    current_call_stack: &mut Vec<EntryPointId>,
    trace: &'a CallTrace,
    show_details: bool,
) -> &'a ExecutionResources {
    current_call_stack.push(EntryPointId::from(
        trace.entry_point.contract_name.clone(),
        trace.entry_point.function_name.clone(),
        trace.entry_point.contract_address.clone(),
        trace.entry_point.entry_point_selector.clone(),
        show_details,
    ));

    let mut children_resources = ExecutionResources::default();
    for sub_trace in &trace.nested_calls {
        children_resources +=
            &collect_samples(samples, current_call_stack, sub_trace, show_details);
    }

    samples.push(Sample {
        call_stack: current_call_stack.iter().map(FunctionName::from).collect(),
        sample_type: SampleType::ContractCall,
        flat_resources: &trace.cumulative_resources - &children_resources,
    });

    current_call_stack.pop();

    &trace.cumulative_resources
}
