use crate::{Artifact, Execution, TaskRef, WaitingCount};
use smallvec::SmallVec;

#[derive(Debug)]
pub enum SuspendCondition<'env> {
    All(WaitingCount),
    Any(SmallVec<[TaskRef<'env>; 2]>),
}

#[derive(Debug)]
pub enum TaskState<'env> {
    Running,
    Suspended(Execution<'env>, SuspendCondition<'env>),
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

    pub fn unwrap_get_execution(self) -> Execution<'env> {
        match self {
            TaskState::Suspended(execution, _condition) => execution,
            _ => panic!("unwrap_get_condition failed!"),
        }
    }
}
