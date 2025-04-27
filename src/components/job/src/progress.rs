use crate::{Artifact, Execution, TaskRef};

pub struct Progress(Progression);

impl Progress {
    #[inline]
    pub fn complete(artifact: Artifact) -> Self {
        Self(Progression::Complete(artifact))
    }

    #[inline]
    pub fn suspend(before: Vec<TaskRef>, execution: impl Into<Execution>) -> Self {
        Self(Progression::Suspend(before, execution.into()))
    }

    #[inline]
    pub fn progression(self) -> Progression {
        self.0
    }
}

impl From<Artifact> for Progress {
    fn from(value: Artifact) -> Self {
        Self(Progression::Complete(value))
    }
}

// Implementation details of Progress with stricter constructors
pub enum Progression {
    Complete(Artifact),
    Suspend(Vec<TaskRef>, Execution),
}
