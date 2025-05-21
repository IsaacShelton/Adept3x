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
        let cyclic = executor.request(Diverge);
        ctx.suspend_on(cyclic);
        Err(Continuation::suspend(Diverge))
    }
}
