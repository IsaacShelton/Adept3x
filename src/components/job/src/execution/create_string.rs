use super::Execute;
use crate::{Artifact, Executor, Progress};

#[derive(Debug)]
pub struct CreateString {
    string: String,
}

impl CreateString {
    pub fn new(string: String) -> Self {
        Self { string }
    }
}

impl<'outside> Execute<'outside> for CreateString {
    fn execute(self, _executor: &Executor<'outside>) -> Progress<'outside> {
        Artifact::String(self.string).into()
    }
}
