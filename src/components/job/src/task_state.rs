use crate::{Artifact, Execution, WaitingCount};

#[derive(Debug)]
pub enum TaskState {
    Running,
    Suspended(Execution, WaitingCount),
    Completed(Artifact),
}

impl TaskState {
    pub fn completed(&self) -> Option<&Artifact> {
        match self {
            TaskState::Running => None,
            TaskState::Suspended(..) => None,
            TaskState::Completed(artifact) => Some(artifact),
        }
    }

    pub fn unwrap_completed(&self) -> &Artifact {
        self.completed().unwrap()
    }

    pub fn unwrap_suspended(self) -> (Execution, WaitingCount) {
        match self {
            TaskState::Suspended(execution, waiting_count) => (execution, waiting_count),
            _ => panic!("unwrap_suspended failed!"),
        }
    }
}
