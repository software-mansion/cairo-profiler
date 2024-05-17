use core::fmt;
use itertools::Itertools;
use std::collections::HashMap;
use std::fmt::Display;

use crate::sierra_loader::CompiledArtifactsPathMap;
use crate::trace_reader::function_trace_builder::collect_profiling_info;
use anyhow::{Context, Result};
use trace_data::{
    CallTrace, CallTraceNode, ContractAddress, EntryPointSelector, ExecutionResources, L1Resources,
};

mod function_trace_builder;

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
    compiled_artifacts_path_map: &CompiledArtifactsPathMap,
    show_details: bool,
) -> Result<Vec<ContractCallSample>> {
    let mut samples = vec![];
    let mut current_path = vec![];

    collect_samples(
        &mut samples,
        &mut current_path,
        trace,
        compiled_artifacts_path_map,
        show_details,
    )?;
    Ok(samples)
}

fn collect_samples<'a>(
    samples: &mut Vec<ContractCallSample>,
    current_call_stack: &mut Vec<EntryPointId>,
    trace: &'a CallTrace,
    compiled_artifacts_path_map: &CompiledArtifactsPathMap,
    show_details: bool,
) -> Result<&'a ExecutionResources> {
    current_call_stack.push(EntryPointId::from(
        trace.entry_point.contract_name.clone(),
        trace.entry_point.function_name.clone(),
        trace.entry_point.contract_address.clone(),
        trace.entry_point.entry_point_selector.clone(),
        show_details,
    ));

    let maybe_entrypoint_steps = if let Some(cairo_execution_info) = &trace.cairo_execution_info {
        let compiled_artifacts = compiled_artifacts_path_map
            .get_sierra_casm_artifacts_for_path(&cairo_execution_info.source_sierra_path);

        let profiling_info = collect_profiling_info(
            &cairo_execution_info.vm_trace,
            compiled_artifacts.sierra.get_program_artifact(),
            &compiled_artifacts.casm_debug_info,
            compiled_artifacts.sierra.was_run_with_header(),
        )?;

        for mut function_stack_trace in profiling_info.functions_stack_traces {
            let mut function_trace = current_call_stack
                .iter()
                .map(FunctionName::from)
                .collect_vec();
            function_trace.append(&mut function_stack_trace.stack_trace);

            samples.push(ContractCallSample {
                call_stack: function_trace,
                measurements: HashMap::from([(
                    MeasurementUnit::from("steps"),
                    MeasurementValue(i64::try_from(function_stack_trace.steps.0).unwrap()),
                )]),
            });
        }
        Some(profiling_info.header_steps)
    } else {
        None
    };

    let mut children_resources = ExecutionResources::default();

    for sub_trace_node in &trace.nested_calls {
        if let CallTraceNode::EntryPointCall(sub_trace) = sub_trace_node {
            children_resources += collect_samples(
                samples,
                current_call_stack,
                sub_trace,
                compiled_artifacts_path_map,
                show_details,
            )?;
        }
    }

    let mut call_resources = &trace.cumulative_resources - &children_resources;

    if let Some(entrypoint_steps) = maybe_entrypoint_steps {
        call_resources.vm_resources.n_steps = entrypoint_steps.0;
    }

    samples.push(ContractCallSample::from(
        current_call_stack.iter().map(FunctionName::from).collect(),
        &call_resources,
        &trace.used_l1_resources,
    ));

    current_call_stack.pop();

    Ok(&trace.cumulative_resources)
}
