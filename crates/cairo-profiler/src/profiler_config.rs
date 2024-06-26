use crate::Cli;

pub struct ProfilerConfig {
    pub show_details: bool,
    pub max_function_stack_trace_depth: usize,
    pub split_generics: bool,
    pub show_inlined_functions: bool,
}

impl From<&Cli> for ProfilerConfig {
    fn from(cli: &Cli) -> ProfilerConfig {
        ProfilerConfig {
            show_details: cli.show_details,
            max_function_stack_trace_depth: cli.max_function_stack_trace_depth,
            split_generics: cli.split_generics,
            show_inlined_functions: cli.show_inlined_functions,
        }
    }
}

pub struct FunctionLevelConfig {
    pub max_function_stack_trace_depth: usize,
    pub split_generics: bool,
    pub show_inlined_functions: bool,
}

impl From<&ProfilerConfig> for FunctionLevelConfig {
    fn from(profiler_config: &ProfilerConfig) -> FunctionLevelConfig {
        FunctionLevelConfig {
            max_function_stack_trace_depth: profiler_config.max_function_stack_trace_depth,
            split_generics: profiler_config.split_generics,
            show_inlined_functions: profiler_config.show_inlined_functions,
        }
    }
}
