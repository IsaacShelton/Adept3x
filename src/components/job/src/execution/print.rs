use super::Execute;
use crate::{Artifact, Executor, Progress, TaskRef};

#[derive(Debug)]
pub struct Print<'outside> {
    message: TaskRef<'outside>,
    indented: bool,
}

impl<'outside> Print<'outside> {
    pub fn new(message: TaskRef<'outside>) -> Self {
        Self {
            message,
            indented: false,
        }
    }
}

impl<'outside> Execute<'outside> for Print<'outside> {
    fn execute(self, executor: &Executor<'outside>) -> Progress<'outside> {
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
