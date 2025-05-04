use super::Execute;
use crate::{Executor, Progress, TaskRef};

#[derive(Debug)]
pub struct Infin {}

impl Infin {
    pub fn new() -> Self {
        Self {}
    }
}

impl<'outside> Execute<'outside> for Infin {
    fn execute(
        self,
        _executor: &Executor<'outside>,
        self_ref: TaskRef<'outside>,
    ) -> Progress<'outside> {
        return Progress::suspend(vec![self_ref], Infin {});
    }
}
