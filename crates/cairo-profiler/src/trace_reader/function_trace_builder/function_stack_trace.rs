use crate::trace_reader::function_name::FunctionName;
use crate::trace_reader::function_trace_builder::Steps;

struct CallStackElement {
    pub function_name: FunctionName,
    /// Steps of the function at the moment of putting it on the stack.
    pub function_steps: Steps,
    /// Consecutive recursive calls to this function that are currently on the stack.
    recursive_calls_count: usize,
}

/// The function call stack of the current function, excluding the current function call.
pub(super) struct CallStack {
    stack: Vec<CallStackElement>,
    /// Tracks the depth of the function call stack, without limit. This is usually equal to
    /// `stack.len()`, but if the actual stack is deeper than `max_function_trace_depth`,
    /// this remains reliable while `stack` does not.
    real_function_stack_depth: usize,
    /// Constant through existence of the object.
    max_function_stack_trace_depth: usize,
}

/// Call that we moved to by exiting a call.
pub(super) enum CurrentCallType {
    /// Regular function call.
    Regular((FunctionName, Steps)),
    /// Function call that has stack trace depth exceeding the limit.
    Hidden,
    /// Recursive function call.
    Recursive,
}

impl CallStack {
    pub fn new(max_function_stack_trace_depth: usize) -> Self {
        Self {
            stack: vec![],
            real_function_stack_depth: 0,
            max_function_stack_trace_depth,
        }
    }

    pub fn enter_function_call(
        &mut self,
        function_name: FunctionName,
        current_function_steps: &mut Steps,
    ) {
        if let Some(stack_element) = self.stack.last_mut() {
            if function_name == stack_element.function_name {
                stack_element.recursive_calls_count += 1;
                return;
            }
        }

        if self.real_function_stack_depth < self.max_function_stack_trace_depth {
            self.stack.push(CallStackElement {
                function_name,
                function_steps: *current_function_steps,
                recursive_calls_count: 0,
            });
            // Reset steps to count new function's steps.
            *current_function_steps = Steps(0);
        }
        self.real_function_stack_depth += 1;
    }

    pub fn exit_function_call(&mut self) -> Option<CurrentCallType> {
        if self.real_function_stack_depth <= self.max_function_stack_trace_depth {
            let mut stack_element = self.stack.pop()?;

            if stack_element.recursive_calls_count > 0 {
                stack_element.recursive_calls_count -= 1;
                self.stack.push(stack_element);

                Some(CurrentCallType::Recursive)
            } else {
                self.real_function_stack_depth -= 1;
                Some(CurrentCallType::Regular((
                    stack_element.function_name,
                    stack_element.function_steps,
                )))
            }
        } else {
            self.real_function_stack_depth -= 1;
            Some(CurrentCallType::Hidden)
        }
    }

    pub fn current_function_names_stack(&self) -> Vec<FunctionName> {
        self.stack
            .iter()
            .map(|stack_element| stack_element.function_name.clone())
            .collect()
    }
}
