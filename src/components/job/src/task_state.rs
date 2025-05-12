use crate::{Artifact, Execution, WaitingCount};

#[derive(Debug)]
pub enum TaskState<'env> {
    Running,
    Suspended(Execution<'env>, WaitingCount),
    Completed(Artifact<'env>),
}

impl<'env> TaskState<'env> {
    pub fn completed(&self) -> Option<&Artifact<'env>> {
        match self {
            TaskState::Running => None,
            TaskState::Suspended(..) => None,
            TaskState::Completed(artifact) => Some(artifact),
        }
    }

    pub fn unwrap_completed(&self) -> &Artifact<'env> {
        self.completed().unwrap()
    }

    pub fn unwrap_suspended(self) -> (Execution<'env>, WaitingCount) {
        match self {
            TaskState::Suspended(execution, waiting_count) => (execution, waiting_count),
            _ => panic!("unwrap_suspended failed!"),
        }
    }
}
