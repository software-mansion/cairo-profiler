use crate::trace_reader::function_trace_builder::Steps;
use crate::trace_reader::functions::FunctionName;

pub struct StackElement {
    pub function_name: FunctionName,
    pub caller_function_steps: Steps,
    recursive_calls_count: usize,
}

pub struct FunctionStack {
    stack: Vec<StackElement>,
    // Tracks the depth of the function stack, without limit. This is usually equal to
    // `function_stack.len()`, but if the actual stack is deeper than `max_stack_trace_depth`,
    // this remains reliable while `function_stack` does not.
    real_function_stack_depth: usize,
    // Constant through existence of the object.
    max_function_trace_depth: usize,
}

pub enum FunctionElement {
    Regular(StackElement),
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
            if function_name == stack_element.function_name {
                stack_element.recursive_calls_count += 1;
                return;
            }
        }

        if self.real_function_stack_depth < self.max_function_trace_depth {
            self.stack.push(StackElement {
                function_name,
                caller_function_steps: *current_function_steps,
                recursive_calls_count: 0,
            });
            // Reset steps to count new function's steps.
            *current_function_steps = Steps(0);
        }
        self.real_function_stack_depth += 1;
    }

    pub fn exit_function_call(&mut self) -> Option<FunctionElement> {
        if self.real_function_stack_depth <= self.max_function_trace_depth {
            let mut stack_element = self.stack.pop()?;

            if stack_element.recursive_calls_count > 0 {
                stack_element.recursive_calls_count -= 1;
                self.stack.push(stack_element);

                Some(FunctionElement::Recursive)
            } else {
                self.real_function_stack_depth -= 1;
                Some(FunctionElement::Regular(stack_element))
            }
        } else {
            self.real_function_stack_depth -= 1;
            Some(FunctionElement::Hidden)
        }
    }

    pub fn build_current_function_stack(&self) -> Vec<FunctionName> {
        self.stack
            .iter()
            .map(|stack_element| stack_element.function_name.clone())
            .collect()
    }
}