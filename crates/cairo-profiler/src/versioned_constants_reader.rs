use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use trace_data::{DeprecatedSyscallSelector, VmExecutionResources};

#[derive(Debug, Serialize, Deserialize)]
pub struct OsResources {
    pub execute_syscalls: HashMap<DeprecatedSyscallSelector, VmExecutionResources>,
}

/// Reads and parses the resource map file at given path
/// It also checks that the file have cost information about all required libfuncs (syscalls)
pub fn read_and_parse_versioned_constants_file(path: &Option<Utf8PathBuf>) -> Result<OsResources> {
    let file_content = match path {
        Some(path) => fs::read_to_string(path)?,
        None => include_str!("../resources/versioned_constants_0_13_2_1.json").to_string(),
    };
    let json_value: Value = serde_json::from_str(&file_content)?;
    let parsed_resources = json_value
        .get("os_resources")
        .context("Field 'os_resources' not found in versioned constants file")?;
    let os_resources: OsResources = serde_json::from_value(parsed_resources.clone())?;

    let missing_libfuncs: Vec<_> = DeprecatedSyscallSelector::all()
        .iter()
        .filter(|&syscall| !os_resources.execute_syscalls.contains_key(syscall))
        .copied()
        .collect();

    if !missing_libfuncs.is_empty() {
        return Err(anyhow::anyhow!(
            "Missing libfuncs in versioned constants file: {:?}",
            missing_libfuncs
        ));
    }

    Ok(os_resources)
}
