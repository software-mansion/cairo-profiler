use std::fs;

use crate::profile_builder::save_profile;
use crate::profile_viewer::{load_profile, print_profile};
use crate::profiler_config::ProfilerConfig;
use crate::sierra_loader::collect_and_compile_all_sierra_programs;
use crate::trace_reader::collect_samples_from_trace;
use crate::versioned_constants_reader::read_and_parse_versioned_constants_file;
use anyhow::{Context, Result};
use cairo_annotations::trace_data::VersionedCallTrace;
use clap::Parser;
use cli::{Cli, Commands};
use profile_builder::build_profile;
#[macro_use]
extern crate prettytable;

mod cli;
mod profile_builder;
mod profile_viewer;
mod profiler_config;
mod sierra_loader;
mod trace_reader;
mod versioned_constants_reader;

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::BuildProfile(build_cli) => {
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
            let profiler_config = ProfilerConfig::from(&build_cli);

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
            save_profile(&build_cli.output_path, &profile)
                .context("Failed to write profile data to file")?;

            if build_cli.view {
                print_profile(&profile, &build_cli.sample, &build_cli.limit)?;
            }

            Ok(())
        }
        Commands::View(view_cli) => {
            let profile = load_profile(&view_cli.path_to_profile)?;
            print_profile(&profile, &view_cli.sample, &view_cli.limit)?;
            Ok(())
        }
    }
}
