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
