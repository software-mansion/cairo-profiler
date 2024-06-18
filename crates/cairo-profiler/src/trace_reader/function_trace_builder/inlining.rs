use std::cmp::max;

use cairo_lang_sierra::program::StatementIdx;
use itertools::Itertools;

use crate::sierra_loader::StatementsFunctionsMap;
use crate::trace_reader::function_name::FunctionName;
use crate::trace_reader::sample::{FunctionCall, InternalFunctionCall};

// TODO: add comments + better names
pub(super) fn build_original_function_stack(
    sierra_statement_idx: StatementIdx,
    maybe_statements_functions_map: Option<&StatementsFunctionsMap>,
    original_call_stack_of_last_non_inlined_function_call: Vec<FunctionCall>,
) -> Vec<FunctionCall> {
    let maybe_original_function_names_stack = maybe_statements_functions_map
        .as_ref()
        .and_then(|statements_functions_map| statements_functions_map.get(sierra_statement_idx));

    if let Some(mappings) = maybe_original_function_names_stack {
        // Statements functions map represents callstack from the least meaningful elements.
        let mappings = mappings.iter().rev().collect_vec();
        construct_original_function_stack(
            original_call_stack_of_last_non_inlined_function_call,
            &mappings,
        )
    } else {
        original_call_stack_of_last_non_inlined_function_call
    }
}

// TODO: test mutually recursive functions (+ inline always with 3 funcs) - add tests!!!
fn construct_original_function_stack(
    original_call_stack_of_last_non_inlined_function_call: Vec<FunctionCall>,
    mappings: &[&FunctionName],
) -> Vec<FunctionCall> {
    let start_index = max(
        original_call_stack_of_last_non_inlined_function_call.len() as i128
            - mappings.len() as i128,
        0,
    )
    .try_into()
    .expect("Non-negative i128 to usize cast should never fail");
    let mut num_of_overlapping_functions = 0;
    for i in start_index..original_call_stack_of_last_non_inlined_function_call.len() {
        let mut overlapped = true;

        for j in 0..original_call_stack_of_last_non_inlined_function_call.len() - i {
            if mappings[j]
                != original_call_stack_of_last_non_inlined_function_call[i + j].function_name()
            {
                overlapped = false;
                break;
            }
        }

        if overlapped {
            num_of_overlapping_functions =
                original_call_stack_of_last_non_inlined_function_call.len() - i;
            break;
        }
    }

    let mut result = original_call_stack_of_last_non_inlined_function_call;

    for &function_name in &mappings[num_of_overlapping_functions..mappings.len()] {
        result.push(FunctionCall::InternalFunctionCall(
            InternalFunctionCall::Inlined(function_name.clone()),
        ));
    }

    result
}
