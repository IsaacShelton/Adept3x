use super::Execute;
use crate::{Artifact, Executor, Progress, TaskRef};

#[derive(Debug)]
pub struct Print<'env> {
    message: TaskRef<'env>,
    indented: bool,
}

impl<'env> Print<'env> {
    pub fn new(message: TaskRef<'env>) -> Self {
        Self {
            message,
            indented: false,
        }
    }
}

impl<'env> Execute<'env> for Print<'env> {
    fn execute(self, executor: &Executor<'env>) -> Progress<'env> {
        if !self.indented {
            print!("> ");

            return Progress::suspend(
                vec![self.message],
                Print {
                    message: self.message,
                    indented: true,
                },
            );
        }

        println!(
            "{}",
            executor.truth.read().unwrap().tasks[self.message]
                .state
                .unwrap_completed()
                .unwrap_string()
        );
        Artifact::Void.into()
    }
}
