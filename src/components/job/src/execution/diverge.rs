use super::Executable;
use crate::{Continuation, ExecutionCtx, Executor};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Diverge;

impl<'env> Executable<'env> for Diverge {
    type Output = ();

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        ctx.suspend_on(executor.request(Diverge));
        Err(Continuation::suspend(Diverge))
    }
}
