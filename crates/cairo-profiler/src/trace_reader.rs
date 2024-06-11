use anyhow::{Context, Result};
use itertools::chain;

use crate::profiler_config::{FunctionLevelConfig, ProfilerConfig};
use crate::sierra_loader::CompiledArtifactsCache;
use crate::trace_reader::function_name::FunctionName;
use crate::trace_reader::function_trace_builder::collect_function_level_profiling_info;

use crate::trace_reader::sample::{AggregatedSample, Function, InternalFunction, Sample};

use trace_data::{CallTrace, CallTraceNode, ExecutionResources};

pub mod function_name;
mod function_trace_builder;
pub mod sample;

pub fn collect_samples_from_trace(
    trace: &CallTrace,
    compiled_artifacts_cache: &CompiledArtifactsCache,
    profiler_config: &ProfilerConfig,
) -> Result<Vec<AggregatedSample>> {
    let mut samples = vec![];
    let mut current_entrypoint_call_stack = vec![];

    collect_samples(
        &mut samples,
        &mut current_entrypoint_call_stack,
        trace,
        compiled_artifacts_cache,
        profiler_config,
    )?;

    Ok(samples.into_iter().map(aggregate_sample).collect())
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

        let function_level_profiling_info = collect_function_level_profiling_info(
            &cairo_execution_info.casm_level_info.vm_trace,
            compiled_artifacts.sierra_program.get_program(),
            &compiled_artifacts.casm_debug_info,
            cairo_execution_info.casm_level_info.run_with_call_header,
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

fn aggregate_sample(sample: Sample) -> AggregatedSample {
    // This vector represent stacks of functions corresponding to single locations.
    // It contains tuples of form (start_index, end_index).
    // A single stack is `&call_stack[start_index..=end_index]`.
    let mut function_stacks_indexes = vec![];

    let mut current_function_stack_start_index = 0;
    for (index, function) in sample.call_stack.iter().enumerate() {
        match function {
            Function::InternalFunction(InternalFunction::NonInlined(_))
            | Function::Entrypoint(_) => {
                if index != 0 {
                    function_stacks_indexes.push((current_function_stack_start_index, index - 1));
                }
                current_function_stack_start_index = index;
            }
            Function::InternalFunction(InternalFunction::Inlined(_)) => {}
        }
    }

    function_stacks_indexes.push((
        current_function_stack_start_index,
        sample.call_stack.len() - 1,
    ));

    let mut aggregated_call_stack = vec![];
    let call_stack_iter = sample.call_stack.into_iter();
    for (start_index, end_index) in function_stacks_indexes {
        aggregated_call_stack.push(
            call_stack_iter
                .clone()
                .take(end_index - start_index + 1)
                .collect(),
        );
    }

    AggregatedSample {
        aggregated_call_stack,
        measurements: sample.measurements,
    }
}
