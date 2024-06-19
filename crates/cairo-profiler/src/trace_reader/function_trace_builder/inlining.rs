use std::cmp::max;

use cairo_lang_sierra::program::StatementIdx;
use itertools::Itertools;

use crate::sierra_loader::StatementsFunctionsMap;
use crate::trace_reader::function_name::FunctionName;
use crate::trace_reader::sample::{FunctionCall, InternalFunctionCall};

// TODO: add comments
pub(super) fn build_original_call_stack_with_inlined_calls(
    sierra_statement_idx: StatementIdx,
    maybe_statements_functions_map: Option<&StatementsFunctionsMap>,
    current_call_stack: Vec<FunctionCall>,
) -> Vec<FunctionCall> {
    let maybe_original_call_stack = maybe_statements_functions_map
        .as_ref()
        .and_then(|statements_functions_map| statements_functions_map.get(sierra_statement_idx));

    if let Some(original_call_stack) = maybe_original_call_stack {
        // Statements functions map represents callstack from the least meaningful elements.
        let original_call_stack = original_call_stack.iter().rev().collect_vec();
        construct_original_call_stack_with_inlined_calls(current_call_stack, &original_call_stack)
    } else {
        current_call_stack
    }
}

// TODO: test mutually recursive functions (+ inline always with 3 funcs) - add tests!!!
fn construct_original_call_stack_with_inlined_calls(
    current_call_stack: Vec<FunctionCall>,
    original_call_stack: &[&FunctionName],
) -> Vec<FunctionCall> {
    let start_index = max(
        current_call_stack.len() as i128 - original_call_stack.len() as i128,
        0,
    )
    .try_into()
    .expect("Non-negative i128 to usize cast should never fail");
    let mut num_of_overlapping_functions = 0;
    for i in start_index..current_call_stack.len() {
        let mut overlapped = true;

        for j in 0..current_call_stack.len() - i {
            if original_call_stack[j] != current_call_stack[i + j].function_name() {
                overlapped = false;
                break;
            }
        }

        if overlapped {
            num_of_overlapping_functions = current_call_stack.len() - i;
            break;
        }
    }

    let mut result = current_call_stack;

    for &function_name in
        &original_call_stack[num_of_overlapping_functions..original_call_stack.len()]
    {
        result.push(FunctionCall::InternalFunctionCall(
            InternalFunctionCall::Inlined(function_name.clone()),
        ));
    }

    result
}
