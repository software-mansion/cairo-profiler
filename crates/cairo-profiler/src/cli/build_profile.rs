use crate::profile_builder::{build_profile, save_profile};
use crate::profile_viewer::print_profile;
use crate::profiler_config::ProfilerConfig;
use crate::sierra_loader::collect_and_compile_all_sierra_programs;
use crate::trace_reader::collect_samples_from_trace;
use crate::versioned_constants_reader::read_and_parse_versioned_constants_file;
use anyhow::{Context, Result};
use cairo_annotations::trace_data::VersionedCallTrace;
use camino::Utf8PathBuf;
use clap::Args;
use std::fs;
use std::num::NonZeroUsize;

#[derive(Args, Debug)]
pub struct BuildProfile {
    /// Path to .json with trace data
    pub path_to_trace_data: Utf8PathBuf,

    /// Path to the output file
    #[arg(short, long, default_value = "profile.pb.gz")]
    pub output_path: Utf8PathBuf,

    /// Show contract addresses and function selectors in a trace tree
    #[arg(long)]
    pub show_details: bool,

    /// Specify maximum depth of function tree in function level profiling.
    /// The is applied per entrypoint - each entrypoint function tree is treated separately.
    #[arg(long, default_value_t = 100)]
    pub max_function_stack_trace_depth: usize,

    /// Split non-inlined generic functions based on the type they were monomorphised with.
    /// E.g. treat `function<felt252>` as different from `function<u8>`.
    #[arg(long)]
    pub split_generics: bool,

    /// Show inlined function in a trace tree. Requires Scarb >= 2.7.0-rc.0 and setting
    /// `unstable-add-statements-functions-debug-info = true` in `[cairo]` section of Scarb.toml.
    #[arg(long)]
    pub show_inlined_functions: bool,

    /// Path to a file, that includes a map with cost of resources like syscalls.
    /// If not provided, the cost map will default to the one used on Starknet 0.13.4.
    /// Files for different Starknet versions can be found in the sequencer repo:
    /// <https://github.com/starkware-libs/sequencer/blob/main/crates/blockifier/resources/>
    #[arg(long)]
    pub versioned_constants_path: Option<Utf8PathBuf>,

    /// View the resulting profile.
    /// To view already-built profile run `cairo-profiler view`.
    #[arg(long)]
    pub view: bool,

    /// Show the sample in the top view.
    /// Requires `--view` flag to be set.
    /// To view already-built profile run `cairo-profiler view`.
    #[arg(long, requires = "view", default_value = "calls")]
    pub sample: String,

    /// Set a limit of viewed nodes.
    /// Requires `--view` flag to be set.
    /// To view already-built profile run `cairo-profiler view`.
    #[arg(long, requires = "view", default_value = "10")]
    pub limit: NonZeroUsize,

    /// Skip nodes matching regex
    /// Requires `--view` flag to be set.
    /// To view already-built profile run `cairo-profiler view`.
    #[arg(long, requires = "view")]
    pub hide: Option<String>,

    /// Show libfuncs in the trace tree.
    #[arg(long)]
    pub show_libfuncs: bool,
}

pub fn run_build_profile(args: &BuildProfile) -> Result<()> {
    let data = fs::read_to_string(&args.path_to_trace_data)
        .context("Failed to read call trace from a file")?;
    let versioned_constants =
        read_and_parse_versioned_constants_file(args.versioned_constants_path.as_ref())
            .context("Failed to get resource map from versioned constants file")?;
    let VersionedCallTrace::V1(serialized_trace) =
        serde_json::from_str(&data).context("Failed to deserialize call trace")?;

    let compiled_artifacts_cache = collect_and_compile_all_sierra_programs(&serialized_trace)?;
    let profiler_config = ProfilerConfig::from(args);

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
        &versioned_constants,
    )?;

    let profile = build_profile(&samples);
    save_profile(&args.output_path, &profile).context("Failed to write profile data to file")?;

    if args.view {
        print_profile(&profile, &args.sample, args.limit, args.hide.as_deref())?;
    }

    Ok(())
}
