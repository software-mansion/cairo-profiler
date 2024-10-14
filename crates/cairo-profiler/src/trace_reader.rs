use anyhow::{Context, Result};
use itertools::chain;

use crate::profiler_config::{FunctionLevelConfig, ProfilerConfig};
use crate::sierra_loader::CompiledArtifactsCache;
use crate::trace_reader::function_name::FunctionName;
use crate::trace_reader::function_trace_builder::collect_function_level_profiling_info;

use crate::trace_reader::sample::{FunctionCall, Sample};
use crate::trace_reader::syscall::collect_syscall_sample;

use trace_data::{CallTrace, CallTraceNode, ExecutionResources, OsResources};

pub mod function_name;
mod function_trace_builder;
pub mod sample;
pub mod syscall;

pub fn collect_samples_from_trace(
    trace: &CallTrace,
    compiled_artifacts_cache: &CompiledArtifactsCache,
    profiler_config: &ProfilerConfig,
    os_resources_map: &OsResources,
) -> Result<Vec<Sample>> {
    let mut samples = vec![];
    let mut current_entrypoint_call_stack = vec![];

    collect_samples(
        &mut samples,
        &mut current_entrypoint_call_stack,
        trace,
        compiled_artifacts_cache,
        profiler_config,
        os_resources_map,
    )?;

    Ok(samples)
}

fn collect_samples<'a>(
    samples: &mut Vec<Sample>,
    current_entrypoint_call_stack: &mut Vec<FunctionCall>,
    trace: &'a CallTrace,
    compiled_artifacts_cache: &CompiledArtifactsCache,
    profiler_config: &ProfilerConfig,
    os_resources_map: &OsResources,
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
            &cairo_execution_info.casm_level_info.vm_trace,
            compiled_artifacts.sierra_program.get_program(),
            &compiled_artifacts.casm_debug_info,
            cairo_execution_info.casm_level_info.run_with_call_header,
            &compiled_artifacts.statements_functions_map,
            &FunctionLevelConfig::from(profiler_config),
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
                os_resources_map,
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

    call_resources
        .syscall_counter
        .iter()
        .filter(|(_, count)| **count != 0)
        .for_each(|(syscall, count)| {
            samples.push(collect_syscall_sample(
                current_entrypoint_call_stack.clone(),
                *syscall,
                *count,
                os_resources_map,
            ));
        });

    current_entrypoint_call_stack.pop();

    Ok(&trace.cumulative_resources)
}
