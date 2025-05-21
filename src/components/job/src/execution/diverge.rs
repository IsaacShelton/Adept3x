use super::Executable;
use crate::{BumpAllocator, Continuation, Executor};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Diverge;

impl<'env> Executable<'env> for Diverge {
    type Output = ();

    fn execute(
        self,
        executor: &Executor<'env>,
        _allocator: &'env BumpAllocator,
    ) -> Result<Self::Output, Continuation<'env>> {
        let cyclic = executor.request(Diverge);
        Err(Continuation::suspend(vec![cyclic.raw_task_ref()], Diverge))
    }
}
