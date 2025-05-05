use crate::{BuildAsgForStruct, Execution, Infin, Task, TaskId, TaskRef};
use arena::Arena;
use derive_more::From;
use std::collections::HashMap;

#[derive(Debug)]
pub struct Truth<'outside> {
    pub tasks: Arena<TaskId, Task<'outside>>,
    pub requests: HashMap<Request<'outside>, TaskRef<'outside>>,
}

impl<'outside> Truth<'outside> {
    pub fn new() -> Self {
        Self {
            tasks: Arena::new(),
            requests: HashMap::new(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, From)]
pub enum Request<'outside> {
    Infin(Infin),
    BuildAsgForStruct(BuildAsgForStruct<'outside>),
}

impl<'outside> Request<'outside> {
    pub fn suspend_on(&self) -> Vec<TaskRef<'outside>> {
        match self {
            Self::Infin(_) => vec![],
            Self::BuildAsgForStruct(inner) => inner.suspend_on(),
        }
    }

    pub fn to_execution(&self) -> Execution<'outside> {
        match self {
            Self::Infin(inner) => inner.clone().into(),
            Self::BuildAsgForStruct(inner) => inner.clone().into(),
        }
    }
}
