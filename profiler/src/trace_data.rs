use serde::{Deserialize, Serialize};
use starknet_api::core::{ClassHash, ContractAddress, EntryPointSelector};
use starknet_api::deprecated_contract_class::EntryPointType;
use starknet_api::transaction::Calldata;

/// Tree structure representing trace of a call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallTrace {
    pub entry_point: CallEntryPoint,
    pub nested_calls: Vec<CallTrace>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CallEntryPoint {
    pub class_hash: Option<ClassHash>,
    pub code_address: Option<ContractAddress>,
    pub entry_point_type: EntryPointType,
    pub entry_point_selector: EntryPointSelector,
    pub calldata: Calldata,
    pub storage_address: ContractAddress,
    pub caller_address: ContractAddress,
    pub call_type: CallType,
    pub initial_gas: u64,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum CallType {
    #[default]
    Call = 0,
    Delegate = 1,
}

// pub fn save_trace_data(summary: &TestCaseSummary<Single>) {
//     if let TestCaseSummary::Passed {
//         name, trace_data, ..
//     } = summary
//     {
//         let serialized_trace =
//             serde_json::to_string(trace_data).expect("Failed to serialize call trace");
//         let dir_to_save_trace = PathBuf::from(TRACE_DIR);
//         fs::create_dir_all(&dir_to_save_trace)
//             .expect("Failed to create a file to save call trace to");

//         fs::write(dir_to_save_trace.join(name), serialized_trace)
//             .expect("Failed to write call trace to a file");
//     }
// }
