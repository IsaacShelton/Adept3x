use crate::{Artifact, Execution, WaitingCount};

#[derive(Debug)]
pub enum TaskState<'outside> {
    Running,
    Suspended(Execution<'outside>, WaitingCount),
    Completed(Artifact<'outside>),
}

impl<'outside> TaskState<'outside> {
    pub fn completed(&self) -> Option<&Artifact<'outside>> {
        match self {
            TaskState::Running => None,
            TaskState::Suspended(..) => None,
            TaskState::Completed(artifact) => Some(artifact),
        }
    }

    pub fn unwrap_completed(&self) -> &Artifact<'outside> {
        self.completed().unwrap()
    }

    pub fn unwrap_suspended(self) -> (Execution<'outside>, WaitingCount) {
        match self {
            TaskState::Suspended(execution, waiting_count) => (execution, waiting_count),
            _ => panic!("unwrap_suspended failed!"),
        }
    }
}
