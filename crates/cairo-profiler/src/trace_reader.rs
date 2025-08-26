use anyhow::{Context, Result};
use itertools::chain;
use std::collections::HashMap;

use crate::profiler_config::{FunctionLevelConfig, ProfilerConfig};
use crate::sierra_loader::CompiledArtifactsCache;
use crate::trace_reader::function_name::FunctionNameExt;
use crate::trace_reader::function_trace_builder::collect_function_level_profiling_info;
use crate::ui;
use cairo_annotations::annotations::profiler::FunctionName;

use crate::trace_reader::sample::{FunctionCall, InternalFunctionCall, Sample};

use crate::trace_reader::function_trace_builder::stack_trace::map_syscall_trace_to_sample;
use crate::versioned_constants_reader::VersionedConstants;
use cairo_annotations::trace_data::{
    CallEntryPoint, CallTraceNode, CallTraceV1, CallType, DeprecatedSyscallSelector,
    EntryPointType, ExecutionResources, SyscallUsage, VmExecutionResources,
};
use indoc::formatdoc;
use std::collections::VecDeque;

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

    if sierra_gas_tracking {
        verify_trace_data_for_l2_gas(trace);
    }

    collect_samples(
        &mut samples,
        &mut current_entrypoint_call_stack,
        trace,
        compiled_artifacts_cache,
        profiler_config,
        versioned_constants,
        sierra_gas_tracking,
        false,
    )?;

    Ok(samples)
}

#[expect(clippy::too_many_lines, clippy::too_many_arguments)]
fn collect_samples<'a>(
    samples: &mut Vec<Sample>,
    current_entrypoint_call_stack: &mut Vec<FunctionCall>,
    trace: &'a CallTraceV1,
    compiled_artifacts_cache: &CompiledArtifactsCache,
    profiler_config: &ProfilerConfig,
    versioned_constants: &VersionedConstants,
    sierra_gas_tracking: bool,
    is_tx_entrypoint: bool,
) -> Result<&'a ExecutionResources> {
    let function_name = FunctionName::from_entry_point_params(
        trace.entry_point.contract_name.clone(),
        trace.entry_point.function_name.clone(),
        trace.entry_point.contract_address.clone(),
        trace.entry_point.entry_point_selector.clone(),
        profiler_config.show_details,
    );
    current_entrypoint_call_stack.push(FunctionCall::EntrypointCall(function_name.clone()));
    let mut children_resources = ExecutionResources::default();

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

        let mut entrypoint_calls = Vec::new();
        let mut calldata_lengths = Vec::new();

        for node in &trace.nested_calls {
            if let CallTraceNode::EntryPointCall(sub_trace) = node {
                if let Some(len) = sub_trace.entry_point.calldata_len {
                    calldata_lengths.push(len);
                }
                entrypoint_calls.push(sub_trace);
            }
        }
        let mut entrypoint_calls = entrypoint_calls.into_iter().peekable();

        let function_level_profiling_info = collect_function_level_profiling_info(
            &compiled_artifacts.sierra_program,
            &compiled_artifacts.casm_debug_info,
            &cairo_execution_info.casm_level_info,
            compiled_artifacts.statements_functions_map.as_ref(),
            &FunctionLevelConfig::from(profiler_config),
            versioned_constants,
            sierra_gas_tracking,
            calldata_lengths,
            &mut VecDeque::from(trace.entry_point.events_summary.clone().unwrap_or_default()),
        );

        let mut trigger_idx = 0;

        while trigger_idx < function_level_profiling_info.nested_call_triggers.len() {
            let trigger = &function_level_profiling_info.nested_call_triggers[trigger_idx];
            let traced_syscall = &trigger.last().unwrap().function_name().0;

            let Some(&sub_trace) = entrypoint_calls.peek() else {
                // There are syscall triggers, but no entrypoints left
                // It is acceptable only for Deploy syscalls, as we filtered out snforge's DeployWithoutConstructor-s
                if traced_syscall == "Deploy" {
                    trigger_idx += 1;
                    continue; // just skip this Deploy trigger and move on
                }
                ui::err(format!(
                    "Found syscall {traced_syscall} in the program trace, that do not have corresponding calls in trace file!"
                ));
                panic!("Too few EntryPointCalls for triggers");
            };

            let expected_syscall = map_entrypoint_to_syscall(&sub_trace.entry_point);
            let is_tx_entrypoint = is_transaction_entrypoint(&function_name);

            if traced_syscall == expected_syscall {
                entrypoint_calls.next();
                trigger_idx += 1;

                let mut triggered_call_stack = current_entrypoint_call_stack.clone();
                triggered_call_stack.extend(trigger.clone());

                children_resources.add_resources(collect_samples(
                    samples,
                    &mut triggered_call_stack,
                    sub_trace,
                    compiled_artifacts_cache,
                    profiler_config,
                    versioned_constants,
                    sierra_gas_tracking,
                    is_tx_entrypoint,
                )?);
            } else if expected_syscall == "Deploy" {
                // snforge can sometimes insert a Deploy nested_call that is not a syscall!
                entrypoint_calls.next();
                children_resources.add_resources(collect_samples(
                    samples,
                    current_entrypoint_call_stack,
                    sub_trace,
                    compiled_artifacts_cache,
                    profiler_config,
                    versioned_constants,
                    sierra_gas_tracking,
                    is_tx_entrypoint,
                )?);
            } else if traced_syscall == "Deploy" {
                // keep looking for matching nested_call
                trigger_idx += 1;
            } else {
                ui::err(format!(
                    "Found syscall {traced_syscall} in the program trace, that do not corresponds to the next call from trace file {:?}!",
                    trace.entry_point
                ));
                panic!("Trigger does not match entrypoint");
            }
        }

        // sanity check: we must be sure all nested_calls were collected into samples
        if entrypoint_calls.next().is_some() {
            ui::err(format!(
                "There are no syscalls left in the program trace, but at least one unhandled call in trace file {:?}!",
                trace.entry_point
            ));
            panic!("Too many EntryPointCalls for triggers");
        }

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
                    false,
                )?);
            }
        }
        None
    };

    let mut call_resources = trace.cumulative_resources.clone();
    call_resources.sub_resources(&children_resources);

    if let Some(entrypoint_steps) = maybe_entrypoint_steps {
        call_resources.vm_resources.n_steps = entrypoint_steps.steps.0;
        call_resources.gas_consumed = Some(entrypoint_steps.sierra_gas_consumed.0.try_into()?);
    }

    // Only applies to traces without explicit Cairo execution info
    if trace.cairo_execution_info.is_none() {
        try_add_syscalls(
            trace,
            samples,
            current_entrypoint_call_stack,
            &function_name,
            versioned_constants,
            sierra_gas_tracking,
        );
    }

    let maybe_entrypoint_l2_gas =
        if is_tx_entrypoint && sierra_gas_tracking && is_priced_transaction(&trace.entry_point) {
            let total_data_size: u64 = (trace.entry_point.calldata_len.unwrap_or_default()
                + trace.entry_point.signature_len.unwrap_or_default())
            .try_into()?;
            let amount: i64 = (versioned_constants
                .archival_data_gas_costs
                .gas_per_data_felt
                * total_data_size)
                .to_integer()
                .try_into()?;

            Some(amount)
        } else {
            None
        };

    samples.push(Sample::from(
        current_entrypoint_call_stack.clone(),
        &call_resources,
        &trace.used_l1_resources,
        maybe_entrypoint_l2_gas,
    ));

    current_entrypoint_call_stack.pop();

    Ok(&trace.cumulative_resources)
}

fn try_add_syscalls(
    trace: &CallTraceV1,
    samples: &mut Vec<Sample>,
    call_stack: &[FunctionCall],
    function_name: &FunctionName,
    versioned_constants: &VersionedConstants,
    sierra_gas_tracking: bool,
) {
    match &trace.cumulative_resources.syscall_counter {
        Some(syscall_counter) => {
            collect_syscall_samples(
                syscall_counter,
                samples,
                call_stack,
                versioned_constants,
                sierra_gas_tracking,
            );
        }
        None => {
            emit_missing_syscall_warning(function_name);
        }
    }
}

fn collect_syscall_samples(
    syscall_counter: &HashMap<DeprecatedSyscallSelector, SyscallUsage>,
    samples: &mut Vec<Sample>,
    base_call_stack: &[FunctionCall],
    versioned_constants: &VersionedConstants,
    sierra_gas_tracking: bool,
) {
    for (selector, usage) in syscall_counter {
        let mut call_stack = base_call_stack.to_vec();
        call_stack.push(FunctionCall::InternalFunctionCall(
            InternalFunctionCall::Syscall(FunctionName(selector.to_string())),
        ));

        let invocations = usage
            .call_count
            .try_into()
            .expect("syscall call count should fit in i64");

        let sample = map_syscall_trace_to_sample(
            call_stack,
            invocations,
            versioned_constants,
            sierra_gas_tracking,
            Some(usage.linear_factor),
            &HashMap::default(),
        );
        samples.push(sample);
    }
}

fn emit_missing_syscall_warning(function_name: &FunctionName) {
    let message = formatdoc! {
        "The trace for {function_name} does not contain syscall counter information. \
         This may lead to inaccurate syscall measurements. \
         Consider using `snforge` >= `0.46.0`."
    };
    ui::warn(message);
}

fn map_entrypoint_to_syscall(entry_point: &CallEntryPoint) -> &str {
    match entry_point.entry_point_type {
        EntryPointType::Constructor => "Deploy",
        EntryPointType::External => match &entry_point.call_type {
            CallType::Call => "CallContract",
            CallType::Delegate => "LibraryCall",
        },
        EntryPointType::L1Handler => "L1Handler",
    }
}

fn verify_trace_data_for_l2_gas(trace: &CallTraceV1) {
    if trace.entry_point.calldata_len.is_none()
        || trace.entry_point.signature_len.is_none()
        || trace.entry_point.events_summary.is_none()
    {
        let message = formatdoc! {
            "The trace file does not contain either one of calldata_len, signature_len or events_summary. \
             This may lead to inaccurate l2 gas measurements. \
             Consider using `snforge` >= `0.49.0`."
        };
        ui::warn(message);
    }
}

fn is_transaction_entrypoint(parent: &FunctionName) -> bool {
    parent
        == &FunctionName(String::from(
            "Contract: SNFORGE_TEST_CODE\nFunction: SNFORGE_TEST_CODE_FUNCTION\n",
        ))
}

fn is_priced_transaction(entrypoint: &CallEntryPoint) -> bool {
    entrypoint.entry_point_type != EntryPointType::Constructor
        && entrypoint.call_type != CallType::Delegate
}
