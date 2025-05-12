use super::Execute;
use crate::{Executor, Progress, TaskRef, prereqs::Prereqs};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Diverge {}

impl Diverge {
    pub fn new() -> Self {
        Self {}
    }
}

impl<'env> Execute<'env> for Diverge {
    fn execute(self, executor: &Executor<'env>) -> Progress<'env> {
        let cyclic = executor.request(Diverge {});
        return Progress::suspend(vec![cyclic], Diverge {});
    }
}

impl<'env> Prereqs<'env> for Diverge {
    fn prereqs(&self) -> Vec<TaskRef<'env>> {
        vec![]
    }
}
