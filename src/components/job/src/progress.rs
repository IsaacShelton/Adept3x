use crate::{Artifact, Execution, TaskRef};
use std::ops::{ControlFlow, FromResidual, Try};

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

impl<'outside> Try for Progress<'outside> {
    type Output = Artifact<'outside>;
    type Residual = Progression<'outside>;

    fn from_output(output: Self::Output) -> Self {
        Self::complete(output)
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self.0 {
            Progression::Complete(artifact) => ControlFlow::Continue(artifact),
            Progression::Suspend(items, execution) => {
                ControlFlow::Break(Progression::Suspend(items, execution))
            }
            Progression::Error(error) => ControlFlow::Break(Progression::Error(error)),
        }
    }
}

impl<'outside> FromResidual<Progression<'outside>> for Progress<'outside> {
    fn from_residual(residual: <Self as Try>::Residual) -> Self {
        residual.into()
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
