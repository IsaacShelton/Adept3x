/*
    ==================  components/job/src/continuation.rs  ===================
    List of (non-completion) continuations that tasks can perform.

    Completion continuations are handled separately by returning Ok(result),
    instead of Err(continuation).
    ---------------------------------------------------------------------------
*/

use crate::Execution;
use derive_more::From;
use diagnostics::ErrorDiagnostic;

#[derive(From)]
pub enum Continuation<'env> {
    // NOTE: To delay waking back up, tasks must be waited on using `ctx.suspend_on` before
    // returning. Usually this is handled indirectly via macro.
    Suspend(Execution<'env>),
    Error(ErrorDiagnostic),
}

impl<'env> Continuation<'env> {
    #[inline]
    pub fn suspend(execution: impl Into<Execution<'env>>) -> Self {
        Self::Suspend(execution.into())
    }
}
