use anyhow::{Context, Result};
use itertools::chain;
use std::collections::HashMap;

use crate::profiler_config::{FunctionLevelConfig, ProfilerConfig};
use crate::sierra_loader::CompiledArtifactsCache;
use crate::trace_reader::function_name::FunctionNameExt;
use crate::trace_reader::function_trace_builder::collect_function_level_profiling_info;
use cairo_annotations::annotations::profiler::FunctionName;

use crate::trace_reader::sample::{FunctionCall, Sample};

use crate::versioned_constants_reader::VersionedConstants;
use cairo_annotations::trace_data::{
    CallTraceNode, CallTraceV1, ExecutionResources, VmExecutionResources,
};

pub mod function_name;
mod function_trace_builder;
pub mod sample;

pub trait ResourcesOperations {
    fn add_resources(&mut self, rhs: &Self);
    fn sub_resources(&mut self, rhs: &Self);
}

impl ResourcesOperations for ExecutionResources {
    fn add_resources(&mut self, other: &ExecutionResources) {
        self.vm_resources.add_resources(&other.vm_resources);
        self.gas_consumed = match (self.gas_consumed, other.gas_consumed) {
            (Some(self_gas), Some(other_gas)) => Some(self_gas + other_gas),
            (Some(self_gas), None) => Some(self_gas),
            (None, Some(other_gas)) => Some(other_gas),
            (None, None) => None,
        };

        if let Some(other_counter) = &other.syscall_counter {
            let self_counter = self.syscall_counter.get_or_insert_with(HashMap::new);
            for (&selector, usage) in other_counter {
                self_counter
                    .entry(selector)
                    .and_modify(|existing| {
                        existing.call_count += usage.call_count;
                        existing.linear_factor += usage.linear_factor;
                    })
                    .or_insert_with(|| usage.clone());
            }
        }
    }

    fn sub_resources(&mut self, other: &ExecutionResources) {
        self.vm_resources.sub_resources(&other.vm_resources);

        if let Some(other_gas) = other.gas_consumed
            && let Some(self_gas) = &mut self.gas_consumed
        {
            *self_gas = self_gas.saturating_sub(other_gas);
        }

        if let Some(self_counter) = &mut self.syscall_counter
            && let Some(other_counter) = &other.syscall_counter
        {
            for (selector, usage) in other_counter {
                if let Some(self_usage) = self_counter.get_mut(selector) {
                    self_usage.call_count = self_usage.call_count.saturating_sub(usage.call_count);
                    self_usage.linear_factor =
                        self_usage.linear_factor.saturating_sub(usage.linear_factor);
                }
            }
            // Remove entries where both values are 0
            self_counter.retain(|_, usage| usage.call_count > 0 || usage.linear_factor > 0);
        }
    }
}
impl ResourcesOperations for VmExecutionResources {
    fn add_resources(&mut self, other: &VmExecutionResources) {
        self.n_steps += other.n_steps;
        self.n_memory_holes += other.n_memory_holes;

        for (key, value) in &other.builtin_instance_counter {
            *self
                .builtin_instance_counter
                .entry(key.clone())
                .or_default() += *value;
        }
    }

    fn sub_resources(&mut self, other: &VmExecutionResources) {
        self.n_steps = self.n_steps.saturating_sub(other.n_steps);
        self.n_memory_holes = self.n_memory_holes.saturating_sub(other.n_memory_holes);

        for (key, value) in &other.builtin_instance_counter {
            if let Some(self_value) = self.builtin_instance_counter.get_mut(key) {
                *self_value = self_value.saturating_sub(*value);
            }
        }
        // Remove entries where the value is 0
        self.builtin_instance_counter.retain(|_, value| *value > 0);
    }
}

pub fn collect_samples_from_trace(
    trace: &CallTraceV1,
    compiled_artifacts_cache: &CompiledArtifactsCache,
    profiler_config: &ProfilerConfig,
    versioned_constants: &VersionedConstants,
) -> Result<Vec<Sample>> {
    let mut samples = vec![];
    let mut current_entrypoint_call_stack = vec![];
    let sierra_gas_tracking: bool = trace.cumulative_resources.gas_consumed.unwrap_or_default() > 0;

    collect_samples(
        &mut samples,
        &mut current_entrypoint_call_stack,
        trace,
        compiled_artifacts_cache,
        profiler_config,
        versioned_constants,
        sierra_gas_tracking,
    )?;

    Ok(samples)
}

fn collect_samples<'a>(
    samples: &mut Vec<Sample>,
    current_entrypoint_call_stack: &mut Vec<FunctionCall>,
    trace: &'a CallTraceV1,
    compiled_artifacts_cache: &CompiledArtifactsCache,
    profiler_config: &ProfilerConfig,
    versioned_constants: &VersionedConstants,
    sierra_gas_tracking: bool,
) -> Result<&'a ExecutionResources> {
    current_entrypoint_call_stack.push(FunctionCall::EntrypointCall(
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

        let function_level_profiling_info = collect_function_level_profiling_info(
            compiled_artifacts.sierra_program.get_program(),
            &compiled_artifacts.casm_debug_info,
            &cairo_execution_info.casm_level_info,
            compiled_artifacts.statements_functions_map.as_ref(),
            &FunctionLevelConfig::from(profiler_config),
            versioned_constants,
            sierra_gas_tracking,
        );

        let mut function_samples = function_level_profiling_info
            .functions_samples
            .into_iter()
            .map(
                |Sample {
                     measurements,
                     call_stack,
                 }| Sample {
                    measurements,
                    call_stack: chain!(current_entrypoint_call_stack.clone(), call_stack).collect(),
                },
            )
            .collect();

        samples.append(&mut function_samples);
        Some(function_level_profiling_info.header_resources)
    } else {
        None
    };

    let mut children_resources = ExecutionResources::default();

    for sub_trace_node in &trace.nested_calls {
        if let CallTraceNode::EntryPointCall(sub_trace) = sub_trace_node {
            children_resources.add_resources(collect_samples(
                samples,
                current_entrypoint_call_stack,
                sub_trace,
                compiled_artifacts_cache,
                profiler_config,
                versioned_constants,
                sierra_gas_tracking,
            )?);
        }
    }

    let mut call_resources = trace.cumulative_resources.clone();
    call_resources.sub_resources(&children_resources);

    if let Some(entrypoint_steps) = maybe_entrypoint_steps {
        call_resources.vm_resources.n_steps = entrypoint_steps.steps.0;
        call_resources.gas_consumed = Some(entrypoint_steps.sierra_gas_consumed.0.try_into()?);
    }

    samples.push(Sample::from(
        current_entrypoint_call_stack.clone(),
        &call_resources,
        &trace.used_l1_resources,
    ));

    current_entrypoint_call_stack.pop();

    Ok(&trace.cumulative_resources)
}
