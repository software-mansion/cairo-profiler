use crate::sierra_loader::StatementsFunctionsMap;
use crate::trace_reader::function_trace_builder::function_stack_trace::FunctionStack;
use crate::trace_reader::function_trace_builder::Steps;
use crate::trace_reader::functions::FunctionName;
use cairo_lang_sierra::program::StatementIdx;
use itertools::Itertools;
use std::cmp::max;
use std::collections::HashMap;

// TODO: refactor
pub(super) fn add_inlined_functions_info(
    sierra_statement_idx: StatementIdx,
    maybe_statements_functions_map: Option<&StatementsFunctionsMap>,
    function_stack: &FunctionStack,
    current_function_name: &FunctionName,
    functions_stack_traces: &mut HashMap<Vec<FunctionName>, Steps>,
    current_function_steps: &mut Steps,
) {
    let maybe_real_function_stack = maybe_statements_functions_map
        .as_ref()
        .and_then(|x| x.get(sierra_statement_idx));

    if let Some(real_function_stack_suffix) = maybe_real_function_stack {
        let real_function_stack_suffix = real_function_stack_suffix
            .iter()
            .rev() // TODO: add comments
            .dedup()
            .map(|x| FunctionName(x.clone()))
            .collect_vec();

        let mut sierra_function_stack = function_stack.build_current_function_stack();
        sierra_function_stack.push(current_function_name.clone());

        let num_of_overlapping_functions =
            find_array_overlap(&sierra_function_stack, &real_function_stack_suffix);

        for func in &real_function_stack_suffix
            [num_of_overlapping_functions..real_function_stack_suffix.len()]
        {
            sierra_function_stack.push(func.clone());
        }

        // TODO: add as inlined function instead (make hashmap key an enum and then reuse the enum in
        //  `ContractCallSample`. Take it into account while building `Profile` for pprof - signalise
        //  that a function was inlined using multiple lines in one location.
        *functions_stack_traces
            .entry(sierra_function_stack)
            .or_insert(Steps(0)) += 1;

        *current_function_steps -= 1;
    }
}

/// Returns number of overlapping elements.
fn find_array_overlap<T: Eq>(base: &[T], overlapping: &[T]) -> usize {
    let start_index = max(base.len() as i128 - overlapping.len() as i128, 0)
        .try_into()
        .expect("Non-negative i128 to usize cast should never fail");
    for i in start_index..base.len() {
        let mut overlapped = true;

        for j in 0..base.len() - i {
            if overlapping[j] != base[i + j] {
                overlapped = false;
                break;
            }
        }

        if overlapped {
            return base.len() - i;
        }
    }

    0
}
