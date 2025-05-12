use super::Execute;
use crate::{Artifact, CreateString, Executor, Progress, TaskRef};

#[derive(Debug)]
pub struct PrintMessage<'env> {
    message: String,
    task_ref: Option<TaskRef<'env>>,
}

impl PrintMessage<'_> {
    pub fn new(message: String) -> Self {
        Self {
            message,
            task_ref: None,
        }
    }
}

impl<'env> Execute<'env> for PrintMessage<'env> {
    fn execute(self, executor: &Executor<'env>) -> Progress<'env> {
        let Some(message_ref) = self.task_ref else {
            let message_ref = executor.push_unique(&[], CreateString::new(self.message));

            return Progress::suspend(
                vec![message_ref],
                PrintMessage {
                    message: "".into(),
                    task_ref: Some(message_ref),
                },
            );
        };

        let content = {
            executor.truth.read().unwrap().tasks[message_ref]
                .state
                .unwrap_completed()
                .unwrap_string()
                .to_string()
        };

        if content.len() < 1000 {
            let message_ref =
                executor.push_unique(&[], CreateString::new(format!("{} {}", content, content)));

            return Progress::suspend(
                vec![message_ref],
                PrintMessage {
                    message: "".into(),
                    task_ref: Some(message_ref),
                },
            );
        }

        println!(
            "{}",
            executor.truth.read().unwrap().tasks[message_ref]
                .state
                .unwrap_completed()
                .unwrap_string()
        );

        Artifact::Void.into()
    }
}
