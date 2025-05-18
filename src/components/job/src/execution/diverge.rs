use super::{Executable, Execution, Spawnable};
use crate::{Continuation, Executor, TaskRef};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Diverge;

impl<'env> Executable<'env> for Diverge {
    type Output = ();

    fn execute(self, executor: &Executor<'env>) -> Result<Self::Output, Continuation<'env>> {
        let cyclic = executor.request(Diverge);
        Err(Continuation::suspend(vec![cyclic.raw_task_ref()], Diverge))
    }
}

impl<'env> Spawnable<'env> for Diverge {
    fn spawn(&self) -> (Vec<TaskRef<'env>>, Execution<'env>) {
        (vec![], self.clone().into())
    }
}
