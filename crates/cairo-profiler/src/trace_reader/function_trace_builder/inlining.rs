use std::cmp::max;

use cairo_lang_sierra::program::StatementIdx;
use itertools::Itertools;

use crate::sierra_loader::StatementsFunctionsMap;
use crate::trace_reader::function_name::FunctionName;
use crate::trace_reader::sample::{FunctionCall, InternalFunctionCall};

pub(super) fn build_original_call_stack_with_inlined_calls(
    sierra_statement_idx: StatementIdx,
    statements_functions_map: Option<&StatementsFunctionsMap>,
    current_call_stack: Vec<FunctionCall>,
) -> Vec<FunctionCall> {
    let maybe_original_call_stack_postfix = statements_functions_map
        .as_ref()
        .and_then(|statements_functions_map| statements_functions_map.get(sierra_statement_idx));

    if let Some(original_call_stack_postfix) = maybe_original_call_stack_postfix {
        // Statements functions map represents callstack from the most nested elements.
        let original_call_stack_postfix = original_call_stack_postfix.iter().rev().collect_vec();
        extend_call_stack_with_inlined_calls(current_call_stack, &original_call_stack_postfix)
    } else {
        current_call_stack
    }
}

fn extend_call_stack_with_inlined_calls(
    current_call_stack: Vec<FunctionCall>,
    original_call_stack_postfix: &[&FunctionName],
) -> Vec<FunctionCall> {
    let num_of_overlapping_calls =
        find_number_of_overlapping_calls(&current_call_stack, original_call_stack_postfix);

    let mut result = current_call_stack;

    for &function_name in
        &original_call_stack_postfix[num_of_overlapping_calls..original_call_stack_postfix.len()]
    {
        result.push(FunctionCall::InternalFunctionCall(
            InternalFunctionCall::Inlined(function_name.clone()),
        ));
    }

    result
}

fn find_number_of_overlapping_calls(
    current_call_stack: &[FunctionCall],
    original_call_stack_postfix: &[&FunctionName],
) -> usize {
    let start_index = max(
        current_call_stack.len() as i128 - original_call_stack_postfix.len() as i128,
        0,
    )
    .try_into()
    .expect("Non-negative i128 to usize cast should never fail");

    // We need to find an overlap between the call stack of a current function and the stack from
    // mappings since there can be multiple non-inlined functions in the former one. This can happen
    // if some generated functions were also not inlined.
    let mut num_of_overlapping_calls = 0;
    for i in start_index..current_call_stack.len() {
        let mut overlap_found = true;

        for j in 0..current_call_stack.len() - i {
            if original_call_stack_postfix[j] != current_call_stack[i + j].function_name() {
                overlap_found = false;
                break;
            }
        }

        if overlap_found {
            num_of_overlapping_calls = current_call_stack.len() - i;
            break;
        }
    }

    num_of_overlapping_calls
}
