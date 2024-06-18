use crate::trace_reader::function_name::FunctionName;

struct CallStackElement {
    pub function_name: FunctionName,
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

impl CallStack {
    pub fn new(max_function_stack_trace_depth: usize) -> Self {
        Self {
            stack: vec![],
            real_function_stack_depth: 0,
            max_function_stack_trace_depth,
        }
    }

    pub fn enter_function_call(&mut self, function_name: FunctionName) {
        if let Some(stack_element) = self.stack.last_mut() {
            if function_name == stack_element.function_name {
                stack_element.recursive_calls_count += 1;
                return;
            }
        }

        if self.real_function_stack_depth < self.max_function_stack_trace_depth {
            self.stack.push(CallStackElement {
                function_name,
                recursive_calls_count: 0,
            });
        }
        self.real_function_stack_depth += 1;
    }

    pub fn exit_function_call(&mut self) -> Option<()> {
        if self.real_function_stack_depth <= self.max_function_stack_trace_depth {
            let mut stack_element = self.stack.pop()?;

            if stack_element.recursive_calls_count > 0 {
                // Recursive function exited.
                stack_element.recursive_calls_count -= 1;
                self.stack.push(stack_element);

                Some(())
            } else {
                // Regular function exited.
                self.real_function_stack_depth -= 1;
                Some(())
            }
        } else {
            // Hidden function exited.
            self.real_function_stack_depth -= 1;
            Some(())
        }
    }
}
