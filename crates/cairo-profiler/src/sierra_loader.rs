use anyhow::{anyhow, Context, Result};
use cairo_lang_sierra::program::{ProgramArtifact, VersionedProgram};
use cairo_lang_sierra_to_casm::compiler::{CairoProgramDebugInfo, SierraToCasmConfig};
use cairo_lang_sierra_to_casm::metadata::calc_metadata;
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_starknet_classes::contract_class::ContractClass;
use camino::{Utf8Path, Utf8PathBuf};
use std::collections::HashMap;
use std::fs;
use trace_data::{CallTrace, CallTraceNode};

/// Map with sierra and casm debug info needed for function level profiling.
/// All paths in the map are absolute paths.
pub struct CompiledArtifactsPathMap(HashMap<Utf8PathBuf, CompiledArtifacts>);

pub struct CompiledArtifacts {
    pub sierra: SierraProgramArtifact,
    pub casm_debug_info: CairoProgramDebugInfo,
}

pub enum SierraProgramArtifact {
    VersionedProgram(ProgramArtifact),
    ContractClass(ProgramArtifact),
}

impl SierraProgramArtifact {
    pub fn get_program_artifact(&self) -> &ProgramArtifact {
        match self {
            SierraProgramArtifact::VersionedProgram(program_artifact)
            | SierraProgramArtifact::ContractClass(program_artifact) => program_artifact,
        }
    }

    pub fn was_run_with_header(&self) -> bool {
        match self {
            SierraProgramArtifact::VersionedProgram(_) => true,
            SierraProgramArtifact::ContractClass(_) => false,
        }
    }
}

impl CompiledArtifactsPathMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn compile_sierra_and_add_compiled_artifacts_to_map(
        &mut self,
        sierra_path: &Utf8Path,
    ) -> Result<()> {
        let absolute_sierra_path = sierra_path
            .canonicalize_utf8()
            .with_context(|| format!("Failed to canonicalize path: {sierra_path}"))?;

        if !self.0.contains_key(&absolute_sierra_path) {
            let raw_sierra = fs::read_to_string(&absolute_sierra_path)?;

            if let Ok(contract_class) = serde_json::from_str::<ContractClass>(&raw_sierra) {
                let program_artifact = ProgramArtifact {
                    program: contract_class
                        .extract_sierra_program()
                        .context("Failed to extract sierra program from contract code")?,
                    debug_info: contract_class.sierra_program_debug_info.clone(),
                };

                let (_casm_contract_class, casm_debug_info) =
                    CasmContractClass::from_contract_class_with_debug_info(
                        contract_class,
                        false,
                        usize::MAX,
                    )?;

                self.0.insert(
                    absolute_sierra_path,
                    CompiledArtifacts {
                        sierra: SierraProgramArtifact::ContractClass(program_artifact),
                        casm_debug_info,
                    },
                );

                return Ok(());
            }

            if let Ok(versioned_program) = serde_json::from_str::<VersionedProgram>(&raw_sierra) {
                let program_artifact = versioned_program
                    .into_v1()
                    .context("Failed to extract program artifact from versioned program. Make sure your versioned program is of version 1")?;

                let casm = cairo_lang_sierra_to_casm::compiler::compile(
                    &program_artifact.program,
                    &calc_metadata(&program_artifact.program, Default::default())
                        .with_context(|| "Failed calculating Sierra variables.")?,
                    SierraToCasmConfig {
                        gas_usage_check: true,
                        max_bytecode_size: usize::MAX,
                    },
                )
                .with_context(|| "Compilation failed.")?;

                self.0.insert(
                    absolute_sierra_path,
                    CompiledArtifacts {
                        sierra: SierraProgramArtifact::VersionedProgram(program_artifact),
                        casm_debug_info: casm.debug_info,
                    },
                );

                return Ok(());
            }

            return Err(anyhow!(
                "Failed to deserialize sierra saved under path: {}",
                absolute_sierra_path
            ));
        }

        Ok(())
    }

    pub fn get_sierra_casm_artifacts_for_path(&self, path: &Utf8Path) -> &CompiledArtifacts {
        self.0
            .get(path)
            .unwrap_or_else(|| panic!("Compiled artifacts not found for path {path}"))
    }
}

pub fn collect_and_compile_all_sierra_programs(
    trace: &CallTrace,
) -> Result<CompiledArtifactsPathMap> {
    let mut compiled_artifacts_path_map = CompiledArtifactsPathMap::new();
    collect_compiled_artifacts(trace, &mut compiled_artifacts_path_map)?;

    Ok(compiled_artifacts_path_map)
}

fn collect_compiled_artifacts(
    trace: &CallTrace,
    compiled_artifacts_path_map: &mut CompiledArtifactsPathMap,
) -> Result<()> {
    if let Some(cairo_execution_info) = &trace.cairo_execution_info {
        compiled_artifacts_path_map.compile_sierra_and_add_compiled_artifacts_to_map(
            &cairo_execution_info.source_sierra_path,
        )?;
    }

    for sub_trace_node in &trace.nested_calls {
        if let CallTraceNode::EntryPointCall(sub_trace) = sub_trace_node {
            collect_compiled_artifacts(sub_trace, compiled_artifacts_path_map)?;
        }
    }

    Ok(())
}
