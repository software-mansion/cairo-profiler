use crate::trace_reader::function_trace_builder::Steps;
use crate::trace_reader::functions::FunctionName;

pub(super) struct Function {
    pub name: FunctionName,
    /// Steps of the function at the moment of putting it on the stack.
    pub steps: Steps,
    /// Consecutive recursive calls to this function that are currently on the stack.
    recursive_calls_count: usize,
}

/// The function stack trace of the current function, excluding the current function.
pub(super) struct FunctionStack {
    stack: Vec<Function>,
    /// Tracks the depth of the function stack, without limit. This is usually equal to
    /// `stack.len()`, but if the actual stack is deeper than `max_function_trace_depth`,
    /// this remains reliable while `stack` does not.
    real_function_stack_depth: usize,
    /// Constant through existence of the object.
    max_function_trace_depth: usize,
}

pub(super) enum FunctionType {
    Regular(Function),
    Hidden,
    Recursive,
}

impl FunctionStack {
    pub fn new(max_function_trace_depth: usize) -> Self {
        Self {
            stack: vec![],
            real_function_stack_depth: 0,
            max_function_trace_depth,
        }
    }

    pub fn enter_function_call(
        &mut self,
        function_name: FunctionName,
        current_function_steps: &mut Steps,
    ) {
        if let Some(stack_element) = self.stack.last_mut() {
            if function_name == stack_element.name {
                stack_element.recursive_calls_count += 1;
                return;
            }
        }

        if self.real_function_stack_depth < self.max_function_trace_depth {
            self.stack.push(Function {
                name: function_name,
                steps: *current_function_steps,
                recursive_calls_count: 0,
            });
            // Reset steps to count new function's steps.
            *current_function_steps = Steps(0);
        }
        self.real_function_stack_depth += 1;
    }

    pub fn exit_function_call(&mut self) -> Option<FunctionType> {
        if self.real_function_stack_depth <= self.max_function_trace_depth {
            let mut stack_element = self.stack.pop()?;

            if stack_element.recursive_calls_count > 0 {
                stack_element.recursive_calls_count -= 1;
                self.stack.push(stack_element);

                Some(FunctionType::Recursive)
            } else {
                self.real_function_stack_depth -= 1;
                Some(FunctionType::Regular(stack_element))
            }
        } else {
            self.real_function_stack_depth -= 1;
            Some(FunctionType::Hidden)
        }
    }

    pub fn build_current_function_stack(&self) -> Vec<FunctionName> {
        self.stack
            .iter()
            .map(|stack_element| stack_element.name.clone())
            .collect()
    }
}
