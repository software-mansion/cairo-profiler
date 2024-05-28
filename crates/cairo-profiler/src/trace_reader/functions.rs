use crate::trace_reader::EntryPointId;
use cairo_lang_sierra::program::{Program, StatementIdx};
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
    pub fn from_sierra_statement_idx(
        statement_idx: StatementIdx,
        sierra_program: &Program,
        split_generics: bool,
    ) -> Self {
        // The `-1` here can't cause an underflow as the statement id of first function's entrypoint is
        // always 0, so it is always on the left side of the partition, thus the partition index is > 0.
        let user_function_idx = sierra_program
            .funcs
            .partition_point(|f| f.entry_point.0 <= statement_idx.0)
            - 1;
        let function_name = sierra_program.funcs[user_function_idx].id.to_string();
        // Remove suffix in case of loop function e.g. `[expr36]`.
        let function_name = RE_LOOP_FUNC.replace(&function_name, "");
        // Remove parameters from monomorphised Cairo generics e.g. `<felt252>`.
        let function_name = if split_generics {
            function_name
        } else {
            RE_MONOMORPHIZATION.replace(&function_name, "")
        };

        Self(function_name.to_string())
    }
}
