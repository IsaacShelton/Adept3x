use crate::{Artifact, Execution, TaskRef};
use std::ops::{ControlFlow, FromResidual, Try};

pub struct Progress<'env>(Progression<'env>);

impl<'env> Progress<'env> {
    #[inline]
    pub fn complete(artifact: Artifact<'env>) -> Self {
        Self(Progression::Complete(artifact))
    }

    #[inline]
    pub fn suspend(before: Vec<TaskRef<'env>>, execution: impl Into<Execution<'env>>) -> Self {
        Self(Progression::Suspend(before, execution.into()))
    }

    #[inline]
    pub fn progression(self) -> Progression<'env> {
        self.0
    }
}

impl<'env> From<Artifact<'env>> for Progress<'env> {
    fn from(value: Artifact<'env>) -> Self {
        Self(Progression::Complete(value))
    }
}

impl<'env> Try for Progress<'env> {
    type Output = Artifact<'env>;
    type Residual = Progression<'env>;

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

impl<'env> FromResidual<Progression<'env>> for Progress<'env> {
    fn from_residual(residual: <Self as Try>::Residual) -> Self {
        residual.into()
    }
}

// Implementation details of Progress with stricter constructors
pub enum Progression<'env> {
    Complete(Artifact<'env>),
    Suspend(Vec<TaskRef<'env>>, Execution<'env>),
    Error(String),
}

impl<'env> From<Progression<'env>> for Progress<'env> {
    fn from(value: Progression<'env>) -> Self {
        Self(value)
    }
}
