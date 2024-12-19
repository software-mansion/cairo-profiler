use std::{
    fs,
    io::{Read, Write},
};

use crate::profiler_config::ProfilerConfig;
use crate::sierra_loader::collect_and_compile_all_sierra_programs;
use crate::trace_reader::collect_samples_from_trace;
use crate::versioned_constants_reader::read_and_parse_versioned_constants_file;
use anyhow::{Context, Result};
use bytes::{Buf, BytesMut};
use cairo_annotations::trace_data::VersionedCallTrace;
use clap::Parser;
use cli::Cli;
use flate2::{bufread::GzEncoder, Compression};
use profile_builder::build_profile;
use prost::Message;

mod cli;
mod profile_builder;
mod profiler_config;
mod sierra_loader;
mod trace_reader;
mod versioned_constants_reader;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        None => {
            let build_cli = &cli.build_profile_args.expect("Failed to parse arguments");
            let data = fs::read_to_string(&build_cli.path_to_trace_data)
                .context("Failed to read call trace from a file")?;
            let os_resources_map = read_and_parse_versioned_constants_file(
                build_cli.versioned_constants_path.as_ref(),
            )
            .context("Failed to get resource map from versioned constants file")?;
            let VersionedCallTrace::V1(serialized_trace) =
                serde_json::from_str(&data).context("Failed to deserialize call trace")?;

            let compiled_artifacts_cache =
                collect_and_compile_all_sierra_programs(&serialized_trace)?;
            let profiler_config = ProfilerConfig::from(build_cli);

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
                &os_resources_map,
            )?;

            let profile = build_profile(&samples);

            if let Some(parent) = build_cli.output_path.parent() {
                fs::create_dir_all(parent)
                    .context("Failed to create parent directories for the output file")?;
            }
            let mut file = fs::File::create(&build_cli.output_path)
                .context("Failed to create the output file")?;

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
        Some(_) => todo!("new subcommands will be added here"),
    }
}
