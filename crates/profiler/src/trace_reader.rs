use core::fmt;
use std::collections::{HashMap, HashSet};
use std::fmt::Display;
use std::ops::Add;

use crate::profile_builder::pprof;
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
    pub fn extract_sample_values(
        &self,
        pprof_samples_units: &[pprof::ValueType],
        context: &ProfilerContext,
    ) -> Vec<i64> {
        let mut sample_values_map: HashMap<&str, i64> = vec![
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
            assert!(sample_values_map.get(&&**builtin).is_none());
            sample_values_map.insert(builtin, i64::try_from(*count).unwrap());
        }

        let syscall_counter_with_string: Vec<_> = self
            .flat_resources
            .syscall_counter
            .iter()
            .map(|(syscall, count)| (format!("{syscall:?}"), *count))
            .collect();
        for (syscall, count) in &syscall_counter_with_string {
            assert!(sample_values_map.get(&&**syscall).is_none());
            sample_values_map.insert(syscall, i64::try_from(*count).unwrap());
        }

        let mut sample_values = vec![];
        for value_type in pprof_samples_units {
            let value_type_str =
                context.string_from_string_id(StringId(u64::try_from(value_type.r#type).unwrap()));
            sample_values.push(*sample_values_map.get(value_type_str).unwrap_or(&0));
        }

        sample_values
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

pub struct SampleUnits(HashSet<String>);

impl SampleUnits {
    pub fn new(mut units: HashSet<String>) -> Self {
        units.extend([
            String::from("calls"),
            String::from("n_steps"),
            String::from("n_memory_holes"),
        ]);
        Self(units)
    }

    pub fn pprof_sample_units(&self, context: &mut ProfilerContext) -> Vec<pprof::ValueType> {
        let mut value_types = vec![];

        for unit in &self.0 {
            let unit_without_underscores = unit.replace('_', " ");
            let unit_without_prefix = if unit_without_underscores.starts_with("n ") {
                unit_without_underscores.strip_prefix("n ").unwrap()
            } else {
                &unit_without_underscores
            };
            let unit_string = " ".to_string().add(unit_without_prefix);

            value_types.push(pprof::ValueType {
                r#type: context.string_id(unit).into(),
                unit: context.string_id(&unit_string).into(),
            });
        }

        value_types
    }
}

pub fn collect_sample_units(samples: &[Sample]) -> SampleUnits {
    let mut units = HashSet::new();
    for sample in samples {
        units.extend(
            sample
                .flat_resources
                .vm_resources
                .builtin_instance_counter
                .keys()
                .cloned(),
        );
        units.extend(
            sample
                .flat_resources
                .syscall_counter
                .keys()
                .map(|x| format!("{x:?}")),
        );
    }
    SampleUnits::new(units)
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
