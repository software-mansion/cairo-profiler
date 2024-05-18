use crate::Cli;

pub struct ProfilerConfig {
    pub show_details: bool,
    pub max_function_trace_depth: usize,
    pub split_generics: bool,
}

impl ProfilerConfig {
    pub fn from_cli(cli: &Cli) -> ProfilerConfig {
        ProfilerConfig {
            show_details: cli.show_details,
            max_function_trace_depth: cli.max_function_trace_depth,
            split_generics: cli.split_generics,
        }
    }
}

pub struct FunctionLevelConfig {
    pub max_function_trace_depth: usize,
    pub split_generics: bool,
}

impl FunctionLevelConfig {
    pub fn from_profiler_config(profiler_config: &ProfilerConfig) -> FunctionLevelConfig {
        FunctionLevelConfig {
            max_function_trace_depth: profiler_config.max_function_trace_depth,
            split_generics: profiler_config.split_generics,
        }
    }
}