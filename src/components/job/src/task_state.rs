/*
    ===================  components/job/src/task_state.rs  ====================
    Defines the different states that a task can be in.
    ---------------------------------------------------------------------------
*/

use crate::{Artifact, Execution, SuspendCondition};

#[derive(Debug, Default)]
pub enum TaskState<'env> {
    #[default]
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

    pub fn unwrap_suspended_execution(self) -> Execution<'env> {
        match self {
            TaskState::Suspended(execution, _condition) => execution,
            _ => panic!("unwrap_suspended_execution failed!"),
        }
    }
}
