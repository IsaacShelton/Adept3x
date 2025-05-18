/*
    ==================  components/job/src/continuation.rs  ===================
    List of (non-completion) continuations that tasks can perform.

    Completion continuations are handled separately by returning Ok(result),
    instead of Err(continuation).
    ---------------------------------------------------------------------------
*/

use crate::{Execution, TaskRef};

pub enum Continuation<'env> {
    Suspend(Vec<TaskRef<'env>>, Execution<'env>),
    Error(String),
}

impl<'env> Continuation<'env> {
    #[inline]
    pub fn suspend(before: Vec<TaskRef<'env>>, execution: impl Into<Execution<'env>>) -> Self {
        Self::Suspend(before, execution.into())
    }

    #[inline]
    pub fn error(message: impl ToString) -> Self {
        Self::Error(message.to_string())
    }
}
