use crate::trace_reader::function_trace_builder::Steps;
use crate::trace_reader::EntryPointId;
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref RE_LOOP_FUNC: Regex = Regex::new(r"\[expr\d*\]")
        .expect("Failed to create regex normalising loop functions names");
    static ref RE_MONOMORPHIZATION: Regex = Regex::new(r"<.*>")
        .expect("Failed to create regex normalising mononorphised generic functions names");
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct FunctionName(pub String);

impl From<&EntryPointId> for FunctionName {
    fn from(value: &EntryPointId) -> Self {
        FunctionName(format!("{value}"))
    }
}

impl FunctionName {
    pub fn to_displayable_function_name(&self, split_generics: bool) -> FunctionName {
        // Remove suffix in case of loop function e.g. `[expr36]`.
        let func_name = RE_LOOP_FUNC.replace(&self.0, "");
        // Remove parameters from monomorphised Cairo generics e.g. `<felt252>`.
        FunctionName(
            if split_generics {
                func_name
            } else {
                RE_MONOMORPHIZATION.replace(&func_name, "")
            }
            .to_string(),
        )
    }
}

pub struct FunctionStackTrace {
    pub stack_trace: Vec<FunctionName>,
    pub steps: Steps,
}

impl FunctionStackTrace {
    pub fn to_displayable_function_stack_trace(&self, split_generics: bool) -> FunctionStackTrace {
        let stack_with_recursive_functions = self
            .stack_trace
            .iter()
            .map(|function_name| function_name.to_displayable_function_name(split_generics))
            .collect_vec();

        // Consolidate recursive function calls into one function call - they mess up the flame graph.
        let displayable_stack_trace = stack_with_recursive_functions.into_iter().dedup().collect();

        FunctionStackTrace {
            stack_trace: displayable_stack_trace,
            steps: self.steps,
        }
    }
}
