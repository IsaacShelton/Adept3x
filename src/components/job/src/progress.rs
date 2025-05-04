use crate::{Artifact, Execution, TaskRef};

pub struct Progress<'outside>(Progression<'outside>);

impl<'outside> Progress<'outside> {
    #[inline]
    pub fn complete(artifact: Artifact<'outside>) -> Self {
        Self(Progression::Complete(artifact))
    }

    #[inline]
    pub fn suspend(
        before: Vec<TaskRef<'outside>>,
        execution: impl Into<Execution<'outside>>,
    ) -> Self {
        Self(Progression::Suspend(before, execution.into()))
    }

    #[inline]
    pub fn progression(self) -> Progression<'outside> {
        self.0
    }
}

impl<'outside> From<Artifact<'outside>> for Progress<'outside> {
    fn from(value: Artifact<'outside>) -> Self {
        Self(Progression::Complete(value))
    }
}

// Implementation details of Progress with stricter constructors
pub enum Progression<'outside> {
    Complete(Artifact<'outside>),
    Suspend(Vec<TaskRef<'outside>>, Execution<'outside>),
    Error(String),
}

impl<'outside> From<Progression<'outside>> for Progress<'outside> {
    fn from(value: Progression<'outside>) -> Self {
        Self(value)
    }
}
