use crate::trace_reader::function_name::FunctionName;
use anyhow::{anyhow, Context, Result};
use cairo_lang_sierra::debug_info::DebugInfo;
use cairo_lang_sierra::program::{Program, ProgramArtifact, StatementIdx, VersionedProgram};
use cairo_lang_sierra_to_casm::compiler::{CairoProgramDebugInfo, SierraToCasmConfig};
use cairo_lang_sierra_to_casm::metadata::calc_metadata;
use cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
use cairo_lang_starknet_classes::contract_class::ContractClass;
use camino::{Utf8Path, Utf8PathBuf};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use trace_data::{CallTrace, CallTraceNode};

/// Map with sierra and casm debug info needed for function level profiling.
/// All paths in the map are absolute paths.
pub struct CompiledArtifactsCache(HashMap<Utf8PathBuf, CompiledArtifacts>);

pub struct CompiledArtifacts {
    pub sierra_program: SierraProgram,
    pub casm_debug_info: CairoProgramDebugInfo,
    pub statements_functions_map: Option<StatementsFunctionsMap>,
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

/// This struct maps sierra statement index to a stack of fully qualified paths of cairo functions
/// consisting of a function which caused the statement to be generated and all functions that were
/// inlined or generated along the way, up to the first non-inlined function from the original code.
/// The map represents the stack from the least meaningful elements.
#[derive(Default, Clone)]
pub struct StatementsFunctionsMap(HashMap<StatementIdx, Vec<FunctionName>>);

impl StatementsFunctionsMap {
    pub fn get(&self, key: StatementIdx) -> Option<&Vec<FunctionName>> {
        self.0.get(&key)
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
) -> Option<StatementsFunctionsMap> {
    maybe_sierra_program_debug_info.and_then(|mut debug_info| {
        debug_info
            .annotations
            .shift_remove("github.com/software-mansion/cairo-profiler")
            .map(get_statements_functions_map)
    })
}

pub fn get_statements_functions_map(mut annotations: Value) -> StatementsFunctionsMap {
    assert!(
        annotations.get("statements_functions").is_some(),
        "Wrong debug info annotations format"
    );
    let statements_functions = annotations["statements_functions"].take();
    let map = serde_json::from_value::<HashMap<StatementIdx, Vec<String>>>(statements_functions)
        .expect("Wrong statements function map format");

    StatementsFunctionsMap(
        map.into_iter()
            .map(|(key, names)| (key, names.into_iter().map(FunctionName).collect()))
            .collect(),
    )
}

pub fn collect_and_compile_all_sierra_programs(
    trace: &CallTrace,
) -> Result<CompiledArtifactsCache> {
    let mut compiled_artifacts_cache = CompiledArtifactsCache::new();
    collect_compiled_artifacts(trace, &mut compiled_artifacts_cache)?;

    Ok(compiled_artifacts_cache)
}

fn collect_compiled_artifacts(
    trace: &CallTrace,
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
