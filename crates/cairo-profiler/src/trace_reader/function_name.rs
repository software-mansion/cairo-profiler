use cairo_annotations::annotations::profiler::FunctionName;
use cairo_annotations::trace_data::{ContractAddress, EntryPointSelector};

pub trait FunctionNameExt {
    fn from_entry_point_params(
        contract_name: Option<String>,
        function_name: Option<String>,
        contract_address: ContractAddress,
        function_selector: EntryPointSelector,
        show_details: bool,
    ) -> FunctionName;
}

impl FunctionNameExt for FunctionName {
    /// `contract_name` and `function_name` are always present (in case they are not in trace we just
    /// set `<unknown>` string).
    /// `address` and `selector` are optional and set if `--show-details` flag is enabled
    /// or names are unknown.
    fn from_entry_point_params(
        contract_name: Option<String>,
        function_name: Option<String>,
        contract_address: ContractAddress,
        function_selector: EntryPointSelector,
        show_details: bool,
    ) -> FunctionName {
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
            Some(address) => format!("Address: {}\n", address.to_fixed_hex_string()),
        };
        let selector = match selector {
            None => String::new(),
            Some(selector) => format!("Selector: {}\n", selector.to_fixed_hex_string()),
        };

        FunctionName(format!(
            "Contract: {contract_name}\n{contract_address}Function: {function_name}\n{selector}",
        ))
    }
}
