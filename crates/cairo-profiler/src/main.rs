use std::{
    fs,
    io::{Read, Write},
};

use crate::profiler_config::ProfilerConfig;
use crate::sierra_loader::collect_and_compile_all_sierra_programs;
use crate::trace_reader::collect_samples_from_trace;
use anyhow::{Context, Result};
use bytes::{Buf, BytesMut};
use camino::Utf8PathBuf;
use clap::Parser;
use flate2::{bufread::GzEncoder, Compression};
use profile_builder::build_profile;
use prost::Message;
use trace_data::CallTrace;

mod profile_builder;
mod profiler_config;
mod sierra_loader;
mod trace_reader;

#[derive(Parser, Debug)]
#[command(version)]
#[clap(name = "cairo-profiler")]
struct Cli {
    /// Path to .json with trace data
    path_to_trace_data: Utf8PathBuf,

    /// Path to the output file
    #[arg(short, long, default_value = "profile.pb.gz")]
    output_path: Utf8PathBuf,

    /// Show contract addresses and function selectors in a trace tree
    #[arg(long)]
    show_details: bool,

    /// Specify maximum depth of function tree in function level profiling.
    /// The is applied per entrypoint - each entrypoint function tree is treated separately.
    #[arg(long, default_value_t = 100)]
    max_function_stack_trace_depth: usize,

    /// Split non-inlined generic functions based on the type they were monomorphised with.
    /// E.g. treat `function<felt252>` as different from `function<u8>`.
    #[arg(long)]
    split_generics: bool,

    /// Show inlined function in a trace tree. Requires Scarb >= 2.7.0-rc.0 and setting
    /// `unstable-add-statements-functions-debug-info = true` in `[cairo]` section of Scarb.toml.
    #[arg(long)]
    show_inlined_functions: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let data = fs::read_to_string(&cli.path_to_trace_data)
        .context("Failed to read call trace from a file")?;
    let serialized_trace: CallTrace =
        serde_json::from_str(&data).context("Failed to deserialize call trace")?;

    let compiled_artifacts_cache = collect_and_compile_all_sierra_programs(&serialized_trace)?;
    let profiler_config = ProfilerConfig::from(&cli);

    if profiler_config.show_inlined_functions
        && !compiled_artifacts_cache.statements_functions_maps_are_present()
    {
        eprintln!(
            "[\x1b[0;33mWARNING\x1b[0m] Mappings used for generating information about \
        inlined functions are missing. Make sure to add this to your Scarb.toml:\n\
        [profile.dev.cairo]\nunstable-add-statements-functions-debug-info = true"
        );
    }

    let samples = collect_samples_from_trace(
        &serialized_trace,
        &compiled_artifacts_cache,
        &profiler_config,
    )?;

    let profile = build_profile(&samples);

    if let Some(parent) = cli.output_path.parent() {
        fs::create_dir_all(parent)
            .context("Failed to create parent directories for the output file")?;
    }
    let mut file = fs::File::create(cli.output_path).context("Failed to create the output file")?;

    let mut buffer = BytesMut::new();
    profile
        .encode(&mut buffer)
        .expect("Failed to encode the profile to the buffer");

    let mut buffer_reader = buffer.reader();
    let mut encoder = GzEncoder::new(&mut buffer_reader, Compression::default());

    let mut encoded = vec![];
    encoder
        .read_to_end(&mut encoded)
        .context("Failed to read bytes from the encoder")?;
    file.write_all(&encoded).unwrap();

    Ok(())
}
