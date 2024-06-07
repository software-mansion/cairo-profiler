use cairo_lang_sierra::program::{Program, StatementIdx};
use lazy_static::lazy_static;
use regex::Regex;
use trace_data::{ContractAddress, EntryPointSelector};

lazy_static! {
    static ref RE_LOOP_FUNC: Regex = Regex::new(r"\[expr\d*\]")
        .expect("Failed to create regex normalising loop functions names");
    static ref RE_MONOMORPHIZATION: Regex = Regex::new(r"<.*>")
        .expect("Failed to create regex normalising mononorphised generic functions names");
}

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct FunctionName(pub String);

impl FunctionName {
    pub fn from_sierra_statement_idx(
        statement_idx: StatementIdx,
        sierra_program: &Program,
        split_generics: bool,
    ) -> Self {
        // The `-1` here can't cause an underflow as the statement id of first function's entrypoint is
        // always 0, so it is always on the left side of the partition, thus the partition index is > 0.
        let function_idx = sierra_program
            .funcs
            .partition_point(|f| f.entry_point.0 <= statement_idx.0)
            - 1;
        let function_name = sierra_program.funcs[function_idx].id.to_string();
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

    /// `contract_name` and `function_name` are always present (in case they are not in trace we just
    /// set `<unknown>` string).
    /// `address` and `selector` are optional and set if `--show-details` flag is enabled
    /// or names are unknown.
    pub fn from_entry_point_params(
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

        let contract_address = match address {
            None => String::new(),
            Some(address) => format!("Address: {address}\n"),
        };
        let selector = match selector {
            None => String::new(),
            Some(selector) => format!("Selector: {selector}\n"),
        };

        FunctionName(format!(
            "Contract: {contract_name}\n{contract_address}Function: {function_name}\n{selector}",
        ))
    }
}
