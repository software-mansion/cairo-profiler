use camino::Utf8PathBuf;
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(version)]
#[clap(name = "cairo-profiler")]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Commands,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Builds the profile from provided trace data
    BuildProfile(BuildProfile),
    /// Views built profile
    View(ViewProfile),
}

// TODO: default?
#[derive(Args, Debug)]
pub(crate) struct BuildProfile {
    /// Path to .json with trace data
    pub(crate) path_to_trace_data: Utf8PathBuf,

    /// Path to the output file
    #[arg(short, long, default_value = "profile.pb.gz")]
    pub(crate) output_path: Utf8PathBuf,

    /// Show contract addresses and function selectors in a trace tree
    #[arg(long)]
    pub(crate) show_details: bool,

    /// Specify maximum depth of function tree in function level profiling.
    /// The is applied per entrypoint - each entrypoint function tree is treated separately.
    #[arg(long, default_value_t = 100)]
    pub(crate) max_function_stack_trace_depth: usize,

    /// Split non-inlined generic functions based on the type they were monomorphised with.
    /// E.g. treat `function<felt252>` as different from `function<u8>`.
    #[arg(long)]
    pub(crate) split_generics: bool,

    /// Show inlined function in a trace tree. Requires Scarb >= 2.7.0-rc.0 and setting
    /// `unstable-add-statements-functions-debug-info = true` in `[cairo]` section of Scarb.toml.
    #[arg(long)]
    pub(crate) show_inlined_functions: bool,

    /// Path to a file, that includes a map with cost of resources like syscalls.
    /// If not provided, the cost map will default to the one used on Starknet 0.13.3.
    /// Files for different Starknet versions can be found in the sequencer repo:
    /// <https://github.com/starkware-libs/sequencer/blob/main/crates/blockifier/resources/>
    #[arg(long)]
    pub(crate) versioned_constants_path: Option<Utf8PathBuf>,

    /// View the resulting profile.
    /// To view already-built profile run `cairo-profiler view`.
    #[arg(long)]
    view: bool,

    /// Show the sample in the top view.
    /// Requires `--view` flag to be set.
    /// To view already-built profile run `cairo-profiler view`.
    #[arg(long, requires = "view", default_value = "steps")]
    sample: String,

    /// Set a limit of viewed nodes.
    /// To show all nodes set this to `0`.
    /// Requires `--view` flag to be set.
    /// To view already-built profile run `cairo-profiler view`.
    #[arg(long, requires = "view", default_value = "10")]
    limit: i32,
}

#[derive(Args)]
pub(crate) struct ViewProfile {
    /// Path to .pb.gz file with profile data.
    pub(crate) path_to_profile: Utf8PathBuf,

    /// Show the sample in the top view.
    /// To get the list of available samples use `--list-samples`.
    #[arg(long, default_value = "steps")]
    pub(crate) sample: String,

    /// List all the samples included in the profile.
    #[arg(short, long)]
    list_samples: bool,

    /// Set a limit of nodes showed in the top view.
    /// To show all nodes set this to `0`.
    #[arg(long, default_value = "10")]
    limit: i32,
}
