use cairo_lang_sierra::program::Program;
use cairo_lang_sierra::program_registry::ProgramRegistry;
use cairo_lang_sierra_to_casm::compiler::{CairoProgramDebugInfo, SierraStatementDebugInfo};
use core::fmt;
use std::collections::HashMap;
use std::fmt::Display;
use std::io::Write;
use tempfile::Builder;
use universal_sierra_compiler_api::{
    AssembledProgramWithDebugInfo, UniversalSierraCompilerCommand,
};

use crate::trace_reader::function_trace_builder::collect_profiling_info;
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
    show_details: bool,
    sierra_code: &Program,
    sierra_contracts: &[Program],
) -> Vec<ContractCallSample> {
    let mut samples = vec![];
    let mut current_path = vec![];

    collect_samples(
        &mut samples,
        &mut current_path,
        trace,
        show_details,
        sierra_code,
        sierra_contracts,
    );
    samples
}

fn collect_samples<'a>(
    // TODO
    samples: &mut Vec<ContractCallSample>,
    current_call_stack: &mut Vec<EntryPointId>,
    trace: &'a CallTrace,
    show_details: bool,
    sierra_code: &Program,
    sierra_contracts: &[Program],
) -> &'a ExecutionResources {
    current_call_stack.push(EntryPointId::from(
        trace.entry_point.contract_name.clone(),
        trace.entry_point.function_name.clone(),
        trace.entry_point.contract_address.clone(),
        trace.entry_point.entry_point_selector.clone(),
        show_details,
    ));

    if trace.entry_point.contract_name == Some("SNFORGE_TEST_CODE".to_string()) {
        let sierra_string_as_bytes = serde_json::to_string(sierra_code).unwrap().into_bytes();
        let mut temp_sierra_file = Builder::new().tempfile().unwrap();
        let _ = temp_sierra_file.write(&sierra_string_as_bytes).unwrap();

        let assembled_with_info_raw = String::from_utf8(
            UniversalSierraCompilerCommand::new()
                .inherit_stderr()
                .args(vec![
                    "compile-raw",
                    "--sierra-path",
                    temp_sierra_file.path().to_str().unwrap(),
                ])
                .command()
                .output()
                .unwrap()
                .stdout,
        )
        .unwrap();
        let casm_program: AssembledProgramWithDebugInfo =
            serde_json::from_str(&assembled_with_info_raw).unwrap();

        let casm_debug_info = CairoProgramDebugInfo {
            sierra_statement_info: casm_program
                .debug_info
                .iter()
                .map(|(offset, idx)| SierraStatementDebugInfo {
                    code_offset: *offset,
                    instruction_idx: *idx,
                })
                .collect(),
        };

        let profiling_info = collect_profiling_info(
            &trace.cairo_execution_info.as_ref().unwrap().vm_trace,
            &casm_debug_info,
            sierra_code,
            &ProgramRegistry::new(sierra_code).unwrap(),
        );

        for (trace, weight) in profiling_info {
            println!("{trace:?} {weight:?}");
        }
    }

    let mut children_resources = ExecutionResources::default();

    for sub_trace_node in &trace.nested_calls {
        if let CallTraceNode::EntryPointCall(sub_trace) = sub_trace_node {
            children_resources += collect_samples(
                samples,
                current_call_stack,
                sub_trace,
                show_details,
                sierra_code,
                sierra_contracts,
            );
        }
    }

    samples.push(ContractCallSample::from(
        current_call_stack.iter().map(FunctionName::from).collect(),
        &(&trace.cumulative_resources - &children_resources),
        &trace.used_l1_resources,
    ));

    current_call_stack.pop();

    &trace.cumulative_resources
}
