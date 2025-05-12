use anyhow::{Context, Result};
use cairo_annotations::trace_data::{DeprecatedSyscallSelector, VmExecutionResources};
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct VersionedConstants {
    pub os_resources: OsResources,
    pub os_constants: OsConstants,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsConstants {
    pub step_gas_cost: u64,
    pub memory_hole_gas_cost: u64,
    pub syscall_base_gas_cost: SyscallBaseGasCost,
    pub builtin_gas_costs: BuiltinGasCosts,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OsResources {
    pub execute_syscalls: HashMap<DeprecatedSyscallSelector, VmExecutionResources>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SyscallBaseGasCost {
    pub step_gas_cost: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BuiltinGasCosts {
    pub range_check: u64,
    pub range_check96: u64,
    pub keccak: u64,
    pub pedersen: u64,
    pub bitwise: u64,
    pub ecop: u64,
    pub poseidon: u64,
    pub add_mod: u64,
    pub mul_mod: u64,
    pub ecdsa: u64,
}

/// Reads and parses the resource map file at given path
/// It also checks that the file have cost information about all required libfuncs (syscalls)
pub fn read_and_parse_versioned_constants_file(
    path: Option<&Utf8PathBuf>,
) -> Result<VersionedConstants> {
    let file_content = match path {
        Some(path) => fs::read_to_string(path).with_context(|| {
            format!("Cannot read versioned constants file at specified path {path}")
        })?,
        None => include_str!("../resources/versioned_constants_0_13_4.json").to_string(),
    };
    let json_value: Value = serde_json::from_str(&file_content)
        .context("Failed to parse versioned constants file content")?;

    let parsed_os_constants = json_value
        .get("os_constants")
        .context("Invalid versioned constants file format: field 'os_constants' not found in versioned constants file")?;
    let os_constants: OsConstants = serde_json::from_value(parsed_os_constants.clone())
        .context("Failed to deserialize 'os_constants' field into OsConstants struct")?;

    let parsed_os_resources = json_value
        .get("os_resources")
        .context("Invalid versioned constants file format: field 'os_resources' not found in versioned constants file")?;
    let os_resources: OsResources = serde_json::from_value(parsed_os_resources.clone())
        .context("Failed to deserialize 'os_resources' field into OsResources struct")?;

    let missing_libfuncs: Vec<_> = DeprecatedSyscallSelector::all()
        .iter()
        .filter(|&syscall| !os_resources.execute_syscalls.contains_key(syscall))
        .copied()
        .collect();

    if !missing_libfuncs.is_empty() {
        return Err(anyhow::anyhow!(
            "Missing libfuncs cost in versioned constants file: {:?}.\n\
            Make sure to include costs of these libfuncs in the aforementioned file.",
            missing_libfuncs
        ));
    }

    Ok(VersionedConstants {
        os_resources,
        os_constants,
    })
}
