use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::{AddAssign, Sub, SubAssign};
use std::str::FromStr;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClassHash(pub String);

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContractAddress(pub String);

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct EntryPointSelector(pub String);

/// Tree structure representing trace of a call.
/// This struct should be serialized and used as an input to cairo-profiler.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallTrace {
    pub entry_point: CallEntryPoint,
    #[serde(rename = "used_execution_resources")]
    pub cumulative_resources: ExecutionResources,
    pub used_l1_resources: L1Resources,
    pub nested_calls: Vec<CallTraceNode>,
    pub cairo_execution_info: Option<CairoExecutionInfo>,
}

/// Struct needed for function level profiling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CairoExecutionInfo {
    /// Path to a file with serialized `ContractClass` or `VersionedProgram`.
    pub source_sierra_path: Utf8PathBuf,
    pub casm_level_info: CasmLevelInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CasmLevelInfo {
    pub run_with_call_header: bool,
    pub vm_trace: Vec<TraceEntry>,
}

/// Enum representing node of a trace of a call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CallTraceNode {
    EntryPointCall(Box<CallTrace>),
    DeployWithoutConstructor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    pub pc: usize,
    pub ap: usize,
    pub fp: usize,
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
    Sha256ProcessBlock,
}

impl DeprecatedSyscallSelector {
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[
            DeprecatedSyscallSelector::CallContract,
            DeprecatedSyscallSelector::DelegateCall,
            DeprecatedSyscallSelector::DelegateL1Handler,
            DeprecatedSyscallSelector::Deploy,
            DeprecatedSyscallSelector::EmitEvent,
            DeprecatedSyscallSelector::GetBlockHash,
            DeprecatedSyscallSelector::GetBlockNumber,
            DeprecatedSyscallSelector::GetBlockTimestamp,
            DeprecatedSyscallSelector::GetCallerAddress,
            DeprecatedSyscallSelector::GetContractAddress,
            DeprecatedSyscallSelector::GetExecutionInfo,
            DeprecatedSyscallSelector::GetSequencerAddress,
            DeprecatedSyscallSelector::GetTxInfo,
            DeprecatedSyscallSelector::GetTxSignature,
            DeprecatedSyscallSelector::Keccak,
            DeprecatedSyscallSelector::LibraryCall,
            DeprecatedSyscallSelector::LibraryCallL1Handler,
            DeprecatedSyscallSelector::ReplaceClass,
            DeprecatedSyscallSelector::Secp256k1Add,
            DeprecatedSyscallSelector::Secp256k1GetPointFromX,
            DeprecatedSyscallSelector::Secp256k1GetXy,
            DeprecatedSyscallSelector::Secp256k1Mul,
            DeprecatedSyscallSelector::Secp256k1New,
            DeprecatedSyscallSelector::Secp256r1Add,
            DeprecatedSyscallSelector::Secp256r1GetPointFromX,
            DeprecatedSyscallSelector::Secp256r1GetXy,
            DeprecatedSyscallSelector::Secp256r1Mul,
            DeprecatedSyscallSelector::Secp256r1New,
            DeprecatedSyscallSelector::SendMessageToL1,
            DeprecatedSyscallSelector::StorageRead,
            DeprecatedSyscallSelector::StorageWrite,
            DeprecatedSyscallSelector::Sha256ProcessBlock,
        ]
    }
}

impl From<DeprecatedSyscallSelector> for String {
    fn from(selector: DeprecatedSyscallSelector) -> Self {
        format!("{selector:?}")
    }
}

impl FromStr for DeprecatedSyscallSelector {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CallContract" => Ok(DeprecatedSyscallSelector::CallContract),
            "DelegateCall" => Ok(DeprecatedSyscallSelector::DelegateCall),
            "DelegateL1Handler" => Ok(DeprecatedSyscallSelector::DelegateL1Handler),
            "Deploy" => Ok(DeprecatedSyscallSelector::Deploy),
            "EmitEvent" => Ok(DeprecatedSyscallSelector::EmitEvent),
            "GetBlockHash" => Ok(DeprecatedSyscallSelector::GetBlockHash),
            "GetBlockNumber" => Ok(DeprecatedSyscallSelector::GetBlockNumber),
            "GetBlockTimestamp" => Ok(DeprecatedSyscallSelector::GetBlockTimestamp),
            "GetCallerAddress" => Ok(DeprecatedSyscallSelector::GetCallerAddress),
            "GetContractAddress" => Ok(DeprecatedSyscallSelector::GetContractAddress),
            "GetExecutionInfo" => Ok(DeprecatedSyscallSelector::GetExecutionInfo),
            "GetSequencerAddress" => Ok(DeprecatedSyscallSelector::GetSequencerAddress),
            "GetTxInfo" => Ok(DeprecatedSyscallSelector::GetTxInfo),
            "GetTxSignature" => Ok(DeprecatedSyscallSelector::GetTxSignature),
            "Keccak" => Ok(DeprecatedSyscallSelector::Keccak),
            "LibraryCall" => Ok(DeprecatedSyscallSelector::LibraryCall),
            "LibraryCallL1Handler" => Ok(DeprecatedSyscallSelector::LibraryCallL1Handler),
            "ReplaceClass" => Ok(DeprecatedSyscallSelector::ReplaceClass),
            "Secp256k1Add" => Ok(DeprecatedSyscallSelector::Secp256k1Add),
            "Secp256k1GetPointFromX" => Ok(DeprecatedSyscallSelector::Secp256k1GetPointFromX),
            "Secp256k1GetXy" => Ok(DeprecatedSyscallSelector::Secp256k1GetXy),
            "Secp256k1Mul" => Ok(DeprecatedSyscallSelector::Secp256k1Mul),
            "Secp256k1New" => Ok(DeprecatedSyscallSelector::Secp256k1New),
            "Secp256r1Add" => Ok(DeprecatedSyscallSelector::Secp256r1Add),
            "Secp256r1GetPointFromX" => Ok(DeprecatedSyscallSelector::Secp256r1GetPointFromX),
            "Secp256r1GetXy" => Ok(DeprecatedSyscallSelector::Secp256r1GetXy),
            "Secp256r1Mul" => Ok(DeprecatedSyscallSelector::Secp256r1Mul),
            "Secp256r1New" => Ok(DeprecatedSyscallSelector::Secp256r1New),
            "SendMessageToL1" => Ok(DeprecatedSyscallSelector::SendMessageToL1),
            "StorageRead" => Ok(DeprecatedSyscallSelector::StorageRead),
            "StorageWrite" => Ok(DeprecatedSyscallSelector::StorageWrite),
            "Sha256ProcessBlock" => Ok(DeprecatedSyscallSelector::Sha256ProcessBlock),
            _ => Err(anyhow::anyhow!("Invalid DeprecatedSyscallSelector: {}", s)),
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct CallEntryPoint {
    pub class_hash: Option<ClassHash>,
    pub entry_point_type: EntryPointType,
    pub entry_point_selector: EntryPointSelector,
    pub contract_address: ContractAddress,
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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub enum EntryPointType {
    #[serde(rename = "CONSTRUCTOR")]
    Constructor,
    #[serde(rename = "EXTERNAL")]
    #[default]
    External,
    #[serde(rename = "L1_HANDLER")]
    L1Handler,
}

impl ExecutionResources {
    #[must_use]
    pub fn gt_eq_than(&self, other: &ExecutionResources) -> bool {
        if self.vm_resources.n_steps < other.vm_resources.n_steps
            || self.vm_resources.n_memory_holes < other.vm_resources.n_memory_holes
        {
            return false;
        }

        let self_builtin_counter = &self.vm_resources.builtin_instance_counter;
        let other_builtin_counter = &other.vm_resources.builtin_instance_counter;
        for (builtin, other_count) in other_builtin_counter {
            let self_count = self_builtin_counter.get(builtin).unwrap_or(&0);
            if self_count < other_count {
                return false;
            }
        }

        let self_builtin_counter = &self.syscall_counter;
        let other_builtin_counter = &other.syscall_counter;
        for (syscall, other_count) in other_builtin_counter {
            let self_count = self_builtin_counter.get(syscall).unwrap_or(&0);
            if self_count < other_count {
                return false;
            }
        }

        true
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct L1Resources {
    pub l2_l1_message_sizes: Vec<usize>,
}
