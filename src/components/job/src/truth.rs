use crate::{Artifact, Pending, Request, Task, TaskId, TaskRef, UnwrapFrom};
use arena::Arena;
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

    pub fn demand<T>(&self, pending: Pending<'env, T>) -> &T
    where
        T: UnwrapFrom<Artifact<'env>>,
    {
        T::unwrap_from(
            self.tasks[pending.raw_task_ref()]
                .completed()
                .as_ref()
                .expect("artifact expected"),
        )
    }

    pub fn expect_artifact(&self, task_ref: TaskRef<'env>) -> &Artifact<'env> {
        self.tasks[task_ref]
            .completed()
            .as_ref()
            .expect("artifact expected")
    }
}
