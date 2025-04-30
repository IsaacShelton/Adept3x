use super::Execute;
use crate::{Artifact, Executor, Progress, TaskRef};

#[derive(Debug)]
pub struct Print {
    message: TaskRef,
    indented: bool,
}

impl Print {
    pub fn new(message: TaskRef) -> Self {
        Self {
            message,
            indented: false,
        }
    }
}

impl Execute for Print {
    fn execute(self, executor: &Executor, _: TaskRef) -> Progress {
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
