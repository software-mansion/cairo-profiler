use itertools::{chain, Itertools};
use std::collections::{HashMap, VecDeque};

use cairo_lang_sierra::program::StatementIdx;

use crate::sierra_loader::StatementsFunctionsMap;
use crate::trace_reader::function_name::FunctionName;
use crate::trace_reader::function_trace_builder::function_stack_trace::CallStack;
use crate::trace_reader::function_trace_builder::Steps;
use crate::trace_reader::sample::{FunctionCall, InternalFunction};

pub(super) fn add_inlined_functions_info(
    sierra_statement_idx: StatementIdx,
    maybe_statements_functions_map: Option<&StatementsFunctionsMap>,
    function_stack: &CallStack,
    current_function_name: &FunctionName,
    functions_traces: &mut HashMap<Vec<FunctionCall>, Steps>,
    current_function_steps: &mut Steps,
) {
    let current_function_names_stack = function_stack.current_function_names_stack();
    let sierra_function_names_stack = chain!(
        current_function_names_stack.iter().collect_vec(),
        [current_function_name]
    )
    .collect_vec();

    // If names on the stack are not unique it means that there is some sort of non-trivial
    // recursiveness that won't be reflected in the mappings.
    if sierra_function_names_stack.iter().unique().count() != sierra_function_names_stack.len() {
        return;
    }

    let maybe_original_function_names_stack = maybe_statements_functions_map
        .as_ref()
        .and_then(|statements_functions_map| statements_functions_map.get(sierra_statement_idx));

    if let Some(original_function_names_stack) = maybe_original_function_names_stack {
        let original_function_names_stack = original_function_names_stack
            .iter()
            .rev() // The mappings from `statements_functions_map` represent callstack from the least meaningful element.
            .dedup()
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

/// Compares original (before inlining) function names stack with sierra function names stack to
/// find out which functions were inlined.
fn build_original_function_stack(
    original_function_names_stack: &[&FunctionName],
    sierra_function_names_stack: &[&FunctionName],
) -> Vec<FunctionCall> {
    let mut result = VecDeque::from(vec![
        FunctionCall::InternalFunctionCall(
            InternalFunction::NonInlined(FunctionName(String::new()))
        );
        original_function_names_stack.len()
    ]);

    let mut function_name_to_index_in_original_stack: HashMap<_, _> = original_function_names_stack
        .iter()
        .enumerate()
        .map(|(index, name)| (*name, index))
        .collect();

    // The first common element in original stack and sierra stack is the first non-inlined
    // original cairo function, so we have to put the previous functions in the result separately.
    let mut first_non_inlined_user_function_index = 0;

    while first_non_inlined_user_function_index < sierra_function_names_stack.len()
        && !function_name_to_index_in_original_stack
            .contains_key(sierra_function_names_stack[first_non_inlined_user_function_index])
    {
        result.push_front(FunctionCall::InternalFunctionCall(
            InternalFunction::NonInlined(
                sierra_function_names_stack[first_non_inlined_user_function_index].clone(),
            ),
        ));
        first_non_inlined_user_function_index += 1;
    }

    for &function_name in &sierra_function_names_stack[first_non_inlined_user_function_index..] {
        let index = function_name_to_index_in_original_stack
            .remove(function_name)
            .expect("Part of function stack from mappings should be a superset of sierra function stack. This is a bug, contact us");

        result[index + first_non_inlined_user_function_index] = FunctionCall::InternalFunctionCall(
            InternalFunction::NonInlined(original_function_names_stack[index].clone()),
        );
    }

    let indices_of_inlined_functions = function_name_to_index_in_original_stack
        .into_values()
        .collect_vec();

    for index in indices_of_inlined_functions {
        result[index + first_non_inlined_user_function_index] = FunctionCall::InternalFunctionCall(
            InternalFunction::Inlined(original_function_names_stack[index].clone()),
        );
    }

    result.into()
}
