use anyhow::{anyhow, Context, Result};
use cairo_annotations::annotations::profiler::{
    ProfilerAnnotationsV1, VersionedProfilerAnnotations,
};
use cairo_annotations::annotations::TryFromDebugInfo;
use cairo_annotations::trace_data::{CallTraceNode, CallTraceV1};
use cairo_lang_sierra::debug_info::DebugInfo;
use cairo_lang_sierra::program::{Program, ProgramArtifact, VersionedProgram};
use cairo_lang_sierra_to_casm::compiler::{CairoProgramDebugInfo, SierraToCasmConfig};
use cairo_lang_sierra_to_casm::metadata::calc_metadata;
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_starknet_classes::contract_class::ContractClass;
use camino::{Utf8Path, Utf8PathBuf};
use std::collections::HashMap;
use std::fs;

/// Map with sierra and casm debug info needed for function level profiling.
/// All paths in the map are absolute paths.
pub struct CompiledArtifactsCache(HashMap<Utf8PathBuf, CompiledArtifacts>);

pub struct CompiledArtifacts {
    pub sierra_program: SierraProgram,
    pub casm_debug_info: CairoProgramDebugInfo,
    pub statements_functions_map: Option<ProfilerAnnotationsV1>,
}

pub enum SierraProgram {
    VersionedProgram(Program),
    ContractClass(Program),
}

impl SierraProgram {
    pub fn get_program(&self) -> &Program {
        match self {
            SierraProgram::VersionedProgram(program) | SierraProgram::ContractClass(program) => {
                program
            }
        }
    }
}

impl CompiledArtifactsCache {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn get_compiled_artifacts_for_path(&self, path: &Utf8Path) -> &CompiledArtifacts {
        self.0
            .get(path)
            .unwrap_or_else(|| panic!("Compiled artifacts not found for path {path}"))
    }

    pub fn statements_functions_maps_are_present(&self) -> bool {
        self.0
            .iter()
            .fold(true, |acc, (_path, compiled_artifacts)| {
                acc && compiled_artifacts.statements_functions_map.is_some()
            })
    }
}

pub fn compile_sierra_and_add_compiled_artifacts_to_cache(
    sierra_path: &Utf8Path,
    compiled_artifacts_cache: &mut CompiledArtifactsCache,
) -> Result<()> {
    let absolute_sierra_path = sierra_path
        .canonicalize_utf8()
        .with_context(|| format!("Failed to canonicalize path: {sierra_path}"))?;

    if !compiled_artifacts_cache
        .0
        .contains_key(&absolute_sierra_path)
    {
        let raw_sierra = fs::read_to_string(&absolute_sierra_path)?;

        if let Ok(contract_class) = serde_json::from_str::<ContractClass>(&raw_sierra) {
            let program = contract_class
                .extract_sierra_program()
                .context("Failed to extract sierra program from contract code")?;

            let statements_functions_map =
                maybe_get_statements_functions_map(contract_class.sierra_program_debug_info);

            let contract_class = ContractClass {
                // Debug info is unused in the compilation. This saves us a costly clone.
                sierra_program_debug_info: None,
                ..contract_class
            };

            let (_casm_contract_class, casm_debug_info) =
                CasmContractClass::from_contract_class_with_debug_info(
                    contract_class,
                    false,
                    usize::MAX,
                )
                .context("Sierra -> CASM compilation failed.")?;

            compiled_artifacts_cache.0.insert(
                absolute_sierra_path,
                CompiledArtifacts {
                    sierra_program: SierraProgram::ContractClass(program),
                    casm_debug_info,
                    statements_functions_map,
                },
            );

            return Ok(());
        }

        if let Ok(versioned_program) = serde_json::from_str::<VersionedProgram>(&raw_sierra) {
            let ProgramArtifact{ program, debug_info} = versioned_program
                .into_v1()
                .context("Failed to extract program artifact from versioned program. Make sure your versioned program is of version 1")?;

            let statements_functions_map = maybe_get_statements_functions_map(debug_info);

            let casm = cairo_lang_sierra_to_casm::compiler::compile(
                &program,
                &calc_metadata(&program, Default::default())
                    .with_context(|| "Failed calculating Sierra variables.")?,
                SierraToCasmConfig {
                    gas_usage_check: true,
                    max_bytecode_size: usize::MAX,
                },
            )
            .context("Sierra -> CASM compilation failed.")?;

            compiled_artifacts_cache.0.insert(
                absolute_sierra_path,
                CompiledArtifacts {
                    sierra_program: SierraProgram::VersionedProgram(program),
                    casm_debug_info: casm.debug_info,
                    statements_functions_map,
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

fn maybe_get_statements_functions_map(
    maybe_sierra_program_debug_info: Option<DebugInfo>,
) -> Option<ProfilerAnnotationsV1> {
    let VersionedProfilerAnnotations::V1(annotations) =
        VersionedProfilerAnnotations::try_from_debug_info(&maybe_sierra_program_debug_info?)
            .ok()?;
    Some(annotations)
}

pub fn collect_and_compile_all_sierra_programs(
    trace: &CallTraceV1,
) -> Result<CompiledArtifactsCache> {
    let mut compiled_artifacts_cache = CompiledArtifactsCache::new();
    collect_compiled_artifacts(trace, &mut compiled_artifacts_cache)?;

    Ok(compiled_artifacts_cache)
}

fn collect_compiled_artifacts(
    trace: &CallTraceV1,
    compiled_artifacts_cache: &mut CompiledArtifactsCache,
) -> Result<()> {
    if let Some(cairo_execution_info) = &trace.cairo_execution_info {
        compile_sierra_and_add_compiled_artifacts_to_cache(
            &cairo_execution_info.source_sierra_path,
            compiled_artifacts_cache,
        )?;
    }

    for sub_trace_node in &trace.nested_calls {
        if let CallTraceNode::EntryPointCall(sub_trace) = sub_trace_node {
            collect_compiled_artifacts(sub_trace, compiled_artifacts_cache)?;
        }
    }

    Ok(())
}
