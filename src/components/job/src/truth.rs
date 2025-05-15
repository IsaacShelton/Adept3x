use crate::{
    Artifact, BuildAsgForStruct, Diverge, EstimateDeclScope, Execution, Task, TaskId, TaskRef,
    prereqs::Prereqs, spawn_execution::SpawnExecution,
};
use arena::Arena;
use derive_more::From;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Truth<'env> {
    pub tasks: Arena<TaskId, Task<'env>>,
    pub requests: HashMap<Request<'env>, TaskRef<'env>>,
}

impl<'env> Truth<'env> {
    pub fn new() -> Self {
        Self {
            tasks: Arena::new(),
            requests: HashMap::new(),
        }
    }

    pub fn expect_artifact(&self, task_ref: TaskRef<'env>) -> &Artifact<'env> {
        self.tasks[task_ref]
            .completed()
            .as_ref()
            .expect("artifact expected")
    }
}

#[derive(Debug, PartialEq, Eq, Hash, From)]
pub enum Request<'env> {
    Diverge(Diverge),
    BuildAsgForStruct(BuildAsgForStruct<'env>),
    EstimateDeclScope(EstimateDeclScope<'env>),
}

// enum_dispatch doesn't support the use case we need for this...
macro_rules! dispatch_to_trait_for_request {
    ($self:expr, $trait:ident, $callee:ident) => {
        match $self {
            Self::Diverge(inner) => $trait::$callee(inner),
            Self::BuildAsgForStruct(inner) => $trait::$callee(inner),
            Self::EstimateDeclScope(inner) => $trait::$callee(inner),
        }
    };
}

impl<'env> Request<'env> {
    pub fn spawn_execution(&self) -> Execution<'env> {
        dispatch_to_trait_for_request!(self, SpawnExecution, spawn_execution)
    }

    pub fn prereqs(&self) -> Vec<TaskRef<'env>> {
        dispatch_to_trait_for_request!(self, Prereqs, prereqs)
    }
}
