use super::Execute;
use crate::{Executor, Progress};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Infin {}

impl Infin {
    pub fn new() -> Self {
        Self {}
    }
}

impl<'outside> Execute<'outside> for Infin {
    fn execute(self, executor: &Executor<'outside>) -> Progress<'outside> {
        let cyclic = executor.request(Infin {});
        return Progress::suspend(vec![cyclic], Infin {});
    }
}
