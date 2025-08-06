/*
    ==================  components/job/src/continuation.rs  ===================
    List of (non-completion) continuations that tasks can perform.

    Completion continuations are handled separately by returning Ok(result),
    instead of Err(continuation).
    ---------------------------------------------------------------------------
*/

use crate::Execution;
use diagnostics::ErrorDiagnostic;

pub enum Continuation<'env> {
    // NOTE: To delay waking back up, tasks must be waited on using `ctx.suspend_on` before
    // returning. Usually this is handled indirectly via macro.
    Suspend(Execution<'env>),
    // NOTE: To prevent immediately waking up when going to sleep with no dependencies,
    // we can use this. Note that suspending on other tasks while pending IO is not supported.
    // Instead, create a separate task to handle the IO and wait for it like any other task.
    PendingIo(Execution<'env>),
    Error(ErrorDiagnostic),
}

impl<'env> Continuation<'env> {
    #[inline]
    pub fn suspend(execution: impl Into<Execution<'env>>) -> Self {
        Self::Suspend(execution.into())
    }
}

impl<'env> From<Result<Execution<'env>, ErrorDiagnostic>> for Continuation<'env> {
    fn from(value: Result<Execution<'env>, ErrorDiagnostic>) -> Self {
        match value {
            Ok(ok) => ok.into(),
            Err(err) => err.into(),
        }
    }
}

impl<'env> From<Execution<'env>> for Continuation<'env> {
    fn from(value: Execution<'env>) -> Self {
        Self::Suspend(value)
    }
}

impl<'env> From<ErrorDiagnostic> for Continuation<'env> {
    fn from(value: ErrorDiagnostic) -> Self {
        Self::Error(value)
    }
}
