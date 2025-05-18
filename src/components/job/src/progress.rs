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
