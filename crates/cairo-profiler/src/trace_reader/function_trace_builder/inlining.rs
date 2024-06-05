use crate::sierra_loader::StatementsFunctionsMap;
use crate::trace_reader::function_name::FunctionName;
use crate::trace_reader::function_trace_builder::function_call_trace::FunctionStack;
use crate::trace_reader::function_trace_builder::{Function, Steps};
use crate::trace_reader::Function::{Inlined, NonInlined};
use cairo_lang_sierra::program::StatementIdx;
use itertools::{chain, Itertools};
use std::collections::HashMap;

// TODO: refactor and optimise (clones)
pub(super) fn add_inlined_functions_info(
    sierra_statement_idx: StatementIdx,
    maybe_statements_functions_map: Option<&StatementsFunctionsMap>,
    function_stack: &FunctionStack,
    current_function_name: &FunctionName,
    functions_traces: &mut HashMap<Vec<Function>, Steps>,
    current_function_steps: &mut Steps,
) {
    let maybe_original_function_names_stack = maybe_statements_functions_map
        .as_ref()
        .and_then(|statements_functions_map| statements_functions_map.get(sierra_statement_idx));

    if let Some(original_function_names_stack) = maybe_original_function_names_stack {
        let original_function_names_stack = original_function_names_stack
            .iter()
            .rev() // TODO: add comments
            .dedup()
            .collect_vec();

        let current_function_names_stack = function_stack.current_function_names_stack();
        let sierra_function_names_stack = chain!(
            current_function_names_stack
                .iter()
                .map(|x| &x.0)
                .collect_vec(),
            [&current_function_name.0]
        )
        .collect_vec();

        let original_function_stack = build_original_function_stack(
            &original_function_names_stack,
            &sierra_function_names_stack,
        );

        *functions_traces
            .entry(original_function_stack)
            .or_insert(Steps(0)) += 1;

        *current_function_steps -= 1;
    }
}

/// Compares original (before inlining) function names stack with sierra function names stack to find
/// out which functions were inlined.
fn build_original_function_stack(
    original_function_names_stack: &[&String],
    sierra_function_names_stack: &[&String],
) -> Vec<Function> {
    let mut result =
        vec![NonInlined(FunctionName(String::new())); original_function_names_stack.len()];

    let mut original_function_indices_map = HashMap::new();
    for (index, item) in original_function_names_stack.iter().enumerate() {
        original_function_indices_map
            .entry(item)
            .or_insert_with(Vec::new)
            .push(index);
    }

    for function_name in sierra_function_names_stack {
        if let Some(indices) = original_function_indices_map.get_mut(function_name) {
            if let Some(index) = indices.pop() {
                result[index] =
                    NonInlined(FunctionName(original_function_names_stack[index].clone()));
            }
        }
    }

    let indices_of_inlined_functions = original_function_indices_map
        .into_iter()
        .flat_map(|(_, indices)| indices)
        .collect_vec();

    for index in indices_of_inlined_functions {
        result[index] = Inlined(FunctionName(original_function_names_stack[index].clone()));
    }

    result
}
