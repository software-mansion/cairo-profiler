use serde::{Deserialize, Serialize};
use starknet_api::core::{ClassHash, ContractAddress, EntryPointSelector};
use starknet_api::deprecated_contract_class::EntryPointType;
use starknet_api::transaction::Calldata;
use std::collections::HashMap;
use std::ops::{AddAssign, Sub, SubAssign};

/// Tree structure representing trace of a call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallTrace {
    pub entry_point: CallEntryPoint,
    #[serde(rename = "used_execution_resources")]
    pub cumulative_resources: ExecutionResources,
    pub used_onchain_data: OnchainData,
    pub nested_calls: Vec<CallTrace>,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub struct ExecutionResources {
    pub vm_resources: VmExecutionResources,
    pub syscall_counter: SyscallCounter,
}

impl AddAssign<&ExecutionResources> for ExecutionResources {
    fn add_assign(&mut self, rhs: &ExecutionResources) {
        self.vm_resources += &rhs.vm_resources;
        for (syscall, count) in &rhs.syscall_counter {
            *self.syscall_counter.entry(*syscall).or_insert(0) += count;
        }
    }
}

impl Sub<&ExecutionResources> for &ExecutionResources {
    type Output = ExecutionResources;

    fn sub(self, rhs: &ExecutionResources) -> Self::Output {
        let mut result = self.clone();
        result.vm_resources -= &rhs.vm_resources;
        for (syscall, count) in &rhs.syscall_counter {
            *result.syscall_counter.entry(*syscall).or_insert(0) -= count;
        }
        result
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, Eq, PartialEq)]
pub struct VmExecutionResources {
    pub n_steps: usize,
    pub n_memory_holes: usize,
    pub builtin_instance_counter: HashMap<String, usize>,
}

impl AddAssign<&VmExecutionResources> for VmExecutionResources {
    fn add_assign(&mut self, rhs: &VmExecutionResources) {
        self.n_steps += rhs.n_steps;
        self.n_memory_holes += rhs.n_memory_holes;
        for (k, v) in &rhs.builtin_instance_counter {
            *self.builtin_instance_counter.entry(k.clone()).or_insert(0) += v;
        }
    }
}

impl SubAssign<&VmExecutionResources> for VmExecutionResources {
    fn sub_assign(&mut self, rhs: &VmExecutionResources) {
        self.n_steps -= rhs.n_steps;
        self.n_memory_holes -= rhs.n_memory_holes;
        for (k, v) in &rhs.builtin_instance_counter {
            let entry = self.builtin_instance_counter.entry(k.clone()).or_insert(0);
            *entry = (*entry).saturating_sub(*v);
        }
    }
}

type SyscallCounter = HashMap<DeprecatedSyscallSelector, usize>;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, Eq, Hash, PartialEq)]
pub enum DeprecatedSyscallSelector {
    CallContract,
    DelegateCall,
    DelegateL1Handler,
    Deploy,
    EmitEvent,
    GetBlockHash,
    GetBlockNumber,
    GetBlockTimestamp,
    GetCallerAddress,
    GetContractAddress,
    GetExecutionInfo,
    GetSequencerAddress,
    GetTxInfo,
    GetTxSignature,
    Keccak,
    LibraryCall,
    LibraryCallL1Handler,
    ReplaceClass,
    Secp256k1Add,
    Secp256k1GetPointFromX,
    Secp256k1GetXy,
    Secp256k1Mul,
    Secp256k1New,
    Secp256r1Add,
    Secp256r1GetPointFromX,
    Secp256r1GetXy,
    Secp256r1Mul,
    Secp256r1New,
    SendMessageToL1,
    StorageRead,
    StorageWrite,
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

    /// Contract name to display instead of contract address
    pub contract_name: Option<String>,
    /// Function name to display instead of entry point selector
    pub function_name: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, Deserialize, Serialize)]
pub enum CallType {
    #[default]
    Call = 0,
    Delegate = 1,
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct OnchainData {
    pub l2_l1_message_sizes: Vec<usize>,
}
