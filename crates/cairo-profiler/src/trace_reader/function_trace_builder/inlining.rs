use std::cmp::max;

use cairo_lang_sierra::program::StatementIdx;
use itertools::Itertools;

use crate::trace_reader::function_trace_builder::function_stack_trace::VecWithLimitedCapacity;
use crate::trace_reader::sample::{FunctionCall, InternalFunctionCall};
use cairo_annotations::annotations::profiler::{FunctionName, ProfilerAnnotationsV1};

pub(super) fn build_original_call_stack_with_inlined_calls(
    sierra_statement_idx: StatementIdx,
    statements_functions_map: Option<&ProfilerAnnotationsV1>,
    current_call_stack: VecWithLimitedCapacity<FunctionCall>,
) -> VecWithLimitedCapacity<FunctionCall> {
    let maybe_original_call_stack_suffix =
        statements_functions_map
            .as_ref()
            .and_then(|statements_functions_map| {
                statements_functions_map
                    .statements_functions
                    .get(&sierra_statement_idx)
            });

    if let Some(original_call_stack_suffix) = maybe_original_call_stack_suffix {
        // Statements functions map represents callstack from the most nested elements.
        let original_call_stack_suffix = original_call_stack_suffix.iter().rev().collect_vec();
        extend_call_stack_with_inlined_calls(current_call_stack, &original_call_stack_suffix)
    } else {
        current_call_stack
    }
}

fn extend_call_stack_with_inlined_calls(
    current_call_stack: VecWithLimitedCapacity<FunctionCall>,
    original_call_stack_suffix: &[&FunctionName],
) -> VecWithLimitedCapacity<FunctionCall> {
    let num_of_overlapping_calls =
        find_number_of_overlapping_calls(&current_call_stack, original_call_stack_suffix);

    let mut result = current_call_stack;

    for &function_name in
        &original_call_stack_suffix[num_of_overlapping_calls..original_call_stack_suffix.len()]
    {
        result.push(FunctionCall::InternalFunctionCall(
            InternalFunctionCall::Inlined(function_name.clone()),
        ));
    }

    result
}

fn find_number_of_overlapping_calls(
    current_call_stack: &VecWithLimitedCapacity<FunctionCall>,
    original_call_stack_suffix: &[&FunctionName],
) -> usize {
    let start_index = max(
        current_call_stack.len() as i128 - original_call_stack_suffix.len() as i128,
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
            if original_call_stack_suffix[j] != current_call_stack[i + j].function_name() {
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
