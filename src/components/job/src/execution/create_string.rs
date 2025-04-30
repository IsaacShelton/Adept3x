use super::Execute;
use crate::{Artifact, Executor, Progress, TaskRef};

#[derive(Debug)]
pub struct CreateString {
    string: String,
}

impl CreateString {
    pub fn new(string: String) -> Self {
        Self { string }
    }
}

impl Execute for CreateString {
    fn execute(self, _executor: &Executor, _: TaskRef) -> Progress {
        Artifact::String(self.string).into()
    }
}
