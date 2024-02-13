use core::fmt;
use std::fmt::Display;

use starknet_api::core::{ContractAddress, EntryPointSelector};

use crate::trace_data::CallTrace;

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct FunctionName(pub String);

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Location(pub Vec<FunctionName>);

impl Location {
    #[inline]
    fn from(s: &[EntryPointId]) -> Location {
        Location(s.iter().map(|c| FunctionName(format!("{}", c))).collect())
    }
}

pub enum SampleType {
    ContractCall,
}

pub struct Sample {
    pub location: Location,
    pub sample_type: SampleType,
}

pub struct EntryPointId {
    address: String,
    selector: String,
    contract_name: Option<String>,
    function_name: Option<String>,
}

impl EntryPointId {
    fn from(
        contract_name: Option<String>,
        function_name: Option<String>,
        contract_address: ContractAddress,
        selector: EntryPointSelector,
    ) -> Self {
        let address_str = format!("0x{}", hex::encode(contract_address.0.key().bytes()));
        let selector_str = format!("{}", selector.0);
        EntryPointId {
            address: address_str,
            selector: selector_str,
            contract_name,
            function_name,
        }
    }
}

impl Display for EntryPointId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let contract_name = self
            .contract_name
            .clone()
            .unwrap_or(String::from("<unknown>"));
        let function_name = self
            .function_name
            .clone()
            .unwrap_or(String::from("<unknown>"));
        write!(
            f,
            "Contract address: {}\n Selector: {}\nContract name: {}\nFunction name: {}\n",
            self.address, self.selector, contract_name, function_name
        )
    }
}

pub fn collect_samples_from_trace(trace: &CallTrace) -> Vec<Sample> {
    let mut samples = vec![];
    let mut current_path = vec![];
    collect_samples(&mut samples, &mut current_path, trace);
    samples
}

fn collect_samples(
    samples: &mut Vec<Sample>,
    current_path: &mut Vec<EntryPointId>,
    trace: &CallTrace,
) {
    current_path.push(EntryPointId::from(
        trace.entry_point.contract_name.clone(),
        trace.entry_point.function_name.clone(),
        trace.entry_point.storage_address,
        trace.entry_point.entry_point_selector,
    ));

    samples.push(Sample {
        location: Location::from(current_path),
        sample_type: SampleType::ContractCall,
    });

    for sub_trace in &trace.nested_calls {
        collect_samples(samples, current_path, sub_trace);
    }

    current_path.pop();
}
