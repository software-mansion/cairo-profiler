use crate::trace_reader::function_name::FunctionName;
use crate::trace_reader::sample::{FunctionCall, InternalFunctionCall};

/// The function call stack of the current function, excluding the current function call.
pub(super) struct CallStack {
    stack: VecWithLimitedCapacity<FunctionCall>,
    /// The last element of this vector is always a number of elements of the stack before the last
    /// function call.
    previous_stack_lengths: Vec<usize>,
}

impl CallStack {
    pub fn new(max_function_stack_trace_depth: usize) -> Self {
        Self {
            stack: VecWithLimitedCapacity::new(max_function_stack_trace_depth),
            previous_stack_lengths: vec![],
        }
    }

    pub fn enter_function_call(&mut self, new_call_stack: VecWithLimitedCapacity<FunctionCall>) {
        self.previous_stack_lengths.push(self.stack.len());

        self.stack = new_call_stack;
    }

    pub fn exit_function_call(&mut self) -> Option<()> {
        let previous_stack_len = self.previous_stack_lengths.pop()?;
        self.stack.truncate(previous_stack_len);
        Some(())
    }

    /// Returns current call stack truncated to `max_function_stack_trace_depth`.
    pub fn current_function_call_stack(
        &self,
        current_function_name: FunctionName,
    ) -> VecWithLimitedCapacity<FunctionCall> {
        let mut current_call_stack = self.stack.clone();

        current_call_stack.push(FunctionCall::InternalFunctionCall(
            InternalFunctionCall::NonInlined(current_function_name),
        ));

        current_call_stack
    }
}

#[derive(Clone)]
pub struct VecWithLimitedCapacity<T> {
    vector: Vec<T>,
    max_capacity: usize,
}

impl<T> VecWithLimitedCapacity<T> {
    pub fn new(max_capacity: usize) -> Self {
        Self {
            vector: vec![],
            max_capacity,
        }
    }

    pub fn from(mut vector: Vec<T>, max_capacity: usize) -> Self {
        vector.truncate(max_capacity);
        Self {
            vector,
            max_capacity,
        }
    }

    pub fn push(&mut self, el: T) {
        if self.vector.len() < self.max_capacity {
            self.vector.push(el);
        }
    }

    pub fn truncate(&mut self, len: usize) {
        self.vector.truncate(len);
    }

    pub fn len(&self) -> usize {
        self.vector.len()
    }

    pub fn deconstruct(self) -> (Vec<T>, usize) {
        (self.vector, self.max_capacity)
    }
}

impl<T> From<VecWithLimitedCapacity<T>> for Vec<T> {
    fn from(value: VecWithLimitedCapacity<T>) -> Self {
        value.vector
    }
}
