use super::Execute;
use crate::{Executor, Progress, TaskRef};

#[derive(Debug)]
pub struct Infin {}

impl Infin {
    pub fn new() -> Self {
        Self {}
    }
}

impl Execute for Infin {
    fn execute(self, _executor: &Executor, self_ref: TaskRef) -> Progress {
        return Progress::suspend(vec![self_ref], Infin {});
    }
}
