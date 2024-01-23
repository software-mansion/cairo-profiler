use core::fmt;
use std::fmt::Display;

// structs copied from blockifier to make CallEntryPoint serializable
use serde::{Deserialize, Serialize};

use crate::trace_data::CallTrace;

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct FunctionName(pub String);

#[derive(Clone, Hash, Eq, PartialEq)]
pub struct Location(pub Vec<FunctionName>);

impl Location {
    #[inline]
    fn from(s: &[ContractId]) -> Location {
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

pub struct ContractId {
    address: String,
    name: Option<String>,
}

impl Display for ContractId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = self.name.clone().unwrap_or(String::from("<unknown>"));
        write!(f, "({}, {})", name, self.address)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum CallType {
    #[default]
    Call = 0,
    Delegate = 1,
}

pub fn collect_samples_from_trace(trace: &CallTrace) -> Vec<Sample> {
    let mut samples = vec![];
    let mut current_path = vec![];
    collect_samples(&mut samples, &mut current_path, trace);
    samples
}

fn collect_samples(
    samples: &mut Vec<Sample>,
    current_path: &mut Vec<ContractId>,
    trace: &CallTrace,
) {
    let contract_id = ContractId {
        name: None,
        address: format!("{:?}", trace.entry_point.storage_address),
    };
    current_path.push(contract_id);

    samples.push(Sample {
        location: Location::from(current_path),
        sample_type: SampleType::ContractCall,
    });

    for sub_trace in &trace.nested_calls {
        collect_samples(samples, current_path, sub_trace);
    }

    current_path.pop();
}
