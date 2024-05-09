use anyhow::{anyhow, Context, Result};
use cairo_lang_sierra::program::{ProgramArtifact, VersionedProgram};
use cairo_lang_sierra_to_casm::compiler::{CairoProgramDebugInfo, SierraToCasmConfig};
use cairo_lang_sierra_to_casm::metadata::calc_metadata;
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_starknet_classes::contract_class::ContractClass;
use camino::{Utf8Path, Utf8PathBuf};
use std::collections::HashMap;
use std::fs;

pub struct CompiledArtifactsPathMap {
    map: HashMap<Utf8PathBuf, CompiledArtifacts>,
}

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
                    debug_info: contract_class.sierra_program_debug_info.clone(),
                };

                let (_casm_contract_class, casm_debug_info) =
                    CasmContractClass::from_contract_class_with_debug_info(
                        contract_class,
                        false,
                        usize::MAX,
                    )?;

                self.map.insert(
                    path,
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

                self.map.insert(
                    path,
                    CompiledArtifacts {
                        sierra: SierraProgramArtifact::VersionedProgram(program_artifact),
                        casm_debug_info: casm.debug_info,
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
