use super::{Executable, Execution, Spawnable};
use crate::{Continuation, Executor, TaskRef};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Print<'env>(pub &'env str);

impl<'env> Executable<'env> for Print<'env> {
    type Output = ();

    fn execute(self, _executor: &Executor<'env>) -> Result<Self::Output, Continuation<'env>> {
        println!("{}", self.0);
        Ok(())
    }
}

impl<'env> Spawnable<'env> for Print<'env> {
    fn spawn(&self) -> (Vec<TaskRef<'env>>, Execution<'env>) {
        (vec![], self.clone().into())
    }
}
