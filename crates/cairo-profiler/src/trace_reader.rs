use std::collections::HashMap;

use crate::profiler_config::{FunctionLevelConfig, ProfilerConfig};
use crate::sierra_loader::CompiledArtifactsCache;
use crate::trace_reader::function_name::FunctionName;
use crate::trace_reader::function_trace_builder::collect_function_level_profiling_info;
use anyhow::{Context, Result};
use trace_data::{CallTrace, CallTraceNode, ExecutionResources, L1Resources};

pub mod function_name;
mod function_trace_builder;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct MeasurementUnit(pub String);

impl From<String> for MeasurementUnit {
    fn from(value: String) -> Self {
        MeasurementUnit(value)
    }
}

#[derive(Debug, Clone)]
pub struct MeasurementValue(pub i64);

pub struct Sample {
    pub call_stack: Vec<Function>,
    pub measurements: HashMap<MeasurementUnit, MeasurementValue>,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum Function {
    Entrypoint(FunctionName),
    InternalFunction(InternalFunction),
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum InternalFunction {
    _Inlined(FunctionName),
    NonInlined(FunctionName),
}

impl Sample {
    pub fn from(
        call_stack: Vec<Function>,
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

pub fn collect_samples_from_trace(
    trace: &CallTrace,
    compiled_artifacts_cache: &CompiledArtifactsCache,
    profiler_config: &ProfilerConfig,
) -> Result<Vec<Sample>> {
    let mut samples = vec![];
    let mut current_entrypoint_call_stack = vec![];

    collect_samples(
        &mut samples,
        &mut current_entrypoint_call_stack,
        trace,
        compiled_artifacts_cache,
        profiler_config,
    )?;
    Ok(samples)
}

fn collect_samples<'a>(
    samples: &mut Vec<Sample>,
    current_entrypoint_call_stack: &mut Vec<Function>,
    trace: &'a CallTrace,
    compiled_artifacts_cache: &CompiledArtifactsCache,
    profiler_config: &ProfilerConfig,
) -> Result<&'a ExecutionResources> {
    current_entrypoint_call_stack.push(Function::Entrypoint(
        FunctionName::from_entry_point_params(
            trace.entry_point.contract_name.clone(),
            trace.entry_point.function_name.clone(),
            trace.entry_point.contract_address.clone(),
            trace.entry_point.entry_point_selector.clone(),
            profiler_config.show_details,
        ),
    ));

    let maybe_entrypoint_steps = if let Some(cairo_execution_info) = &trace.cairo_execution_info {
        let absolute_source_sierra_path = cairo_execution_info
            .source_sierra_path
            .canonicalize_utf8()
            .with_context(|| {
                format!(
                    "Failed to canonicalize path: {}",
                    cairo_execution_info.source_sierra_path
                )
            })?;

        let compiled_artifacts =
            compiled_artifacts_cache.get_compiled_artifacts_for_path(&absolute_source_sierra_path);

        let mut function_level_profiling_info = collect_function_level_profiling_info(
            &cairo_execution_info.casm_level_info.vm_trace,
            compiled_artifacts.sierra_program.get_program(),
            &compiled_artifacts.casm_debug_info,
            cairo_execution_info.casm_level_info.run_with_call_header,
            &FunctionLevelConfig::from(profiler_config),
        );

        samples.append(&mut function_level_profiling_info.functions_samples);
        Some(function_level_profiling_info.header_steps)
    } else {
        None
    };

    let mut children_resources = ExecutionResources::default();

    for sub_trace_node in &trace.nested_calls {
        if let CallTraceNode::EntryPointCall(sub_trace) = sub_trace_node {
            children_resources += collect_samples(
                samples,
                current_entrypoint_call_stack,
                sub_trace,
                compiled_artifacts_cache,
                profiler_config,
            )?;
        }
    }

    let mut call_resources = &trace.cumulative_resources - &children_resources;

    if let Some(entrypoint_steps) = maybe_entrypoint_steps {
        call_resources.vm_resources.n_steps = entrypoint_steps.0;
    }

    samples.push(Sample::from(
        current_entrypoint_call_stack.clone(),
        &call_resources,
        &trace.used_l1_resources,
    ));

    current_entrypoint_call_stack.pop();

    Ok(&trace.cumulative_resources)
}
