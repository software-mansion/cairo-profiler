use anyhow::{Context, Result};
use cairo_annotations::trace_data::{DeprecatedSyscallSelector, VmExecutionResources};
use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct OsResources {
    pub execute_syscalls: HashMap<DeprecatedSyscallSelector, VmExecutionResources>,
}

/// Reads and parses the resource map file at given path
/// It also checks that the file have cost information about all required libfuncs (syscalls)
pub fn read_and_parse_versioned_constants_file(path: Option<&Utf8PathBuf>) -> Result<OsResources> {
    let file_content = match path {
        Some(path) => fs::read_to_string(path).with_context(|| {
            format!("Cannot read versioned constants file at specified path {path}")
        })?,
        None => include_str!("../resources/versioned_constants_0_13_4.json").to_string(),
    };
    let json_value: Value = serde_json::from_str(&file_content)
        .context("Failed to parse versioned constants file content")?;
    let parsed_resources = json_value
        .get("os_resources")
        .context("Invalid versioned constants file format: field 'os_resources' not found in versioned constants file")?;
    let os_resources: OsResources = serde_json::from_value(parsed_resources.clone())
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

    Ok(os_resources)
}
