use crate::cli::build_profile::BuildProfile;
use crate::trace_reader::function_name::ExternalTool;

pub struct ProfilerConfig {
    pub show_details: bool,
    pub max_function_stack_trace_depth: usize,
    pub split_generics: bool,
    pub show_inlined_functions: bool,
    pub show_libfuncs: bool,
    pub cairo_enable_gas: bool,
    pub external_tool: ExternalTool,
}

impl ProfilerConfig {
    pub(crate) fn new(
        cli: &BuildProfile,
        cairo_enable_gas: bool,
        external_tool: ExternalTool,
    ) -> ProfilerConfig {
        ProfilerConfig {
            show_details: cli.show_details,
            max_function_stack_trace_depth: cli.max_function_stack_trace_depth,
            split_generics: cli.split_generics,
            show_inlined_functions: cli.show_inlined_functions,
            show_libfuncs: cli.show_libfuncs,
            cairo_enable_gas,
            external_tool,
        }
    }
}

pub struct FunctionLevelConfig {
    pub max_function_stack_trace_depth: usize,
    pub split_generics: bool,
    pub show_inlined_functions: bool,
    pub show_libfuncs: bool,
}

impl From<&ProfilerConfig> for FunctionLevelConfig {
    fn from(profiler_config: &ProfilerConfig) -> FunctionLevelConfig {
        FunctionLevelConfig {
            max_function_stack_trace_depth: profiler_config.max_function_stack_trace_depth,
            split_generics: profiler_config.split_generics,
            show_inlined_functions: profiler_config.show_inlined_functions,
            show_libfuncs: profiler_config.show_libfuncs,
        }
    }
}
