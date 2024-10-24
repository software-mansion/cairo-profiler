use anyhow::{anyhow, Context, Result};
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::{from_str, from_value, Value};
use std::collections::HashMap;
use std::fs;
use trace_data::{DeprecatedSyscallSelector, VmExecutionResources};

#[derive(Debug, Serialize, Deserialize)]
pub struct OsResources {
    pub execute_syscalls: HashMap<DeprecatedSyscallSelector, VmExecutionResources>,
}

pub fn read_and_parse_versioned_constants_file(path: &Option<Utf8PathBuf>) -> Result<OsResources> {
    let file_content = match path {
        Some(path) => fs::read_to_string(path)?,
        // include_str requires a string literal
        None => include_str!("../resources/versioned_constants_0_13_2_1.json").to_string(),
    };
    let json_value: Value = from_str(&file_content)?;
    let parsed_resources = json_value
        .get("os_resources")
        .context("Field 'os_resources' not found in versioned constants file")?;
    let os_resources: OsResources = from_value(parsed_resources.clone())?;
    Ok(os_resources)
}

pub fn map_syscall_name_to_selector(syscall: &str) -> Result<DeprecatedSyscallSelector> {
    match syscall {
        "call_contract_syscall" => Ok(DeprecatedSyscallSelector::CallContract),
        "deploy_syscall" => Ok(DeprecatedSyscallSelector::Deploy),
        "emit_event_syscall" => Ok(DeprecatedSyscallSelector::EmitEvent),
        "get_block_hash_syscall" => Ok(DeprecatedSyscallSelector::GetBlockHash),
        "get_execution_info_syscall" | "get_execution_info_v2_syscall" => {
            Ok(DeprecatedSyscallSelector::GetExecutionInfo)
        }
        "keccak_syscall" => Ok(DeprecatedSyscallSelector::Keccak),
        "library_call_syscall" => Ok(DeprecatedSyscallSelector::LibraryCall),
        "replace_class_syscall" => Ok(DeprecatedSyscallSelector::ReplaceClass),
        "send_message_to_l1_syscall" => Ok(DeprecatedSyscallSelector::SendMessageToL1),
        "storage_read_syscall" => Ok(DeprecatedSyscallSelector::StorageRead),
        "storage_write_syscall" => Ok(DeprecatedSyscallSelector::StorageWrite),
        "sha256_process_block_syscall" => Ok(DeprecatedSyscallSelector::Sha256ProcessBlock),
        _ => Err(anyhow!(
            "Missing mapping for {syscall:?} - used resources values may not be complete"
        )),
    }
}
