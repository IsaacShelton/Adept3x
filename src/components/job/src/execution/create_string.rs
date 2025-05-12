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

impl<'env> Execute<'env> for CreateString {
    fn execute(self, _executor: &Executor<'env>) -> Progress<'env> {
        Artifact::String(self.string).into()
    }
}
