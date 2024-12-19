use camino::Utf8PathBuf;
use clap::Args;

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
    /// If not provided, the cost map will default to the one used on Starknet 0.13.3.
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
    #[arg(long, requires = "view", default_value = "steps")]
    pub sample: String,

    /// Set a limit of viewed nodes.
    /// Requires `--view` flag to be set.
    /// To view already-built profile run `cairo-profiler view`.
    #[arg(long, requires = "view", default_value = "10")]
    pub limit: usize,
}
