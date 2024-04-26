use anyhow::{anyhow, Context, Result};
use cairo_lang_sierra::program::{ProgramArtifact, VersionedProgram};
use cairo_lang_sierra_to_casm::compiler::{CairoProgramDebugInfo, SierraStatementDebugInfo};
use cairo_lang_starknet_classes::contract_class::ContractClass;
use camino::{Utf8Path, Utf8PathBuf};
use std::collections::HashMap;
use std::fs;
use universal_sierra_compiler_api::{compile_sierra_to_casm, AssembledProgramWithDebugInfo};

pub struct CompiledArtifactsPathMap {
    map: HashMap<Utf8PathBuf, CompiledArtifacts>,
}

pub struct CompiledArtifacts {
    pub sierra: ProgramArtifact,
    pub casm_debug_info: CairoProgramDebugInfo,
}

impl CompiledArtifactsPathMap {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn try_create_compiled_artifacts(&mut self, path: &Utf8Path) -> Result<()> {
        let path = path
            .canonicalize_utf8()
            .with_context(|| format!("Failed to canonicalize path: {path}"))?;

        if !self.map.contains_key(&path) {
            let raw_sierra = fs::read_to_string(&path)?;

            if let Ok(contract_class) = serde_json::from_str::<ContractClass>(&raw_sierra) {
                let program_artifact = ProgramArtifact {
                    program: contract_class
                        .extract_sierra_program()
                        .context("Failed to extract sierra program from contract code")?,
                    debug_info: contract_class.sierra_program_debug_info,
                };

                let casm = compile_sierra_to_casm(&program_artifact.program)?;

                self.map.insert(
                    path,
                    CompiledArtifacts {
                        sierra: program_artifact,
                        casm_debug_info: extract_casm_debug_info(casm),
                    },
                );

                return Ok(());
            }

            if let Ok(versioned_program) = serde_json::from_str::<VersionedProgram>(&raw_sierra) {
                let program_artifact = versioned_program
                    .into_v1()
                    .context("Failed to extract program artifact from versioned program. Make sure your versioned program is of version 1")?;

                let casm = compile_sierra_to_casm(&program_artifact.program)?;

                self.map.insert(
                    path,
                    CompiledArtifacts {
                        sierra: program_artifact,
                        casm_debug_info: extract_casm_debug_info(casm),
                    },
                );

                return Ok(());
            }

            return Err(anyhow!(
                "Failed to deserialize sierra saved under path: {}",
                path
            ));
        }

        Ok(())
    }

    pub fn get_sierra_casm_artifacts_for_path(&self, path: &Utf8Path) -> &CompiledArtifacts {
        self.map
            .get(path)
            .unwrap_or_else(|| panic!("Compiled artifacts not found for path {path}"))
    }
}

fn extract_casm_debug_info(casm: AssembledProgramWithDebugInfo) -> CairoProgramDebugInfo {
    CairoProgramDebugInfo {
        sierra_statement_info: casm
            .debug_info
            .into_iter()
            .map(|(offset, idx)| SierraStatementDebugInfo {
                code_offset: offset,
                instruction_idx: idx,
            })
            .collect(),
    }
}
