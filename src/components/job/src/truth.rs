use crate::{Artifact, Pending, Request, Task, TaskId, TaskRef, UnwrapFrom};
use arena::Arena;
use std::collections::HashMap;
use std_ext::BoxedSlice;

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

    pub fn ask<T>(&self, pending: Pending<'env, T>) -> Option<&T>
    where
        T: UnwrapFrom<Artifact<'env>>,
    {
        self.tasks[pending.raw_task_ref()]
            .completed()
            .as_ref()
            .map(|artifact| T::unwrap_from(artifact))
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

    pub fn demand_many<T>(
        &self,
        pending_list: impl Iterator<Item = Pending<'env, T>>,
    ) -> BoxedSlice<T>
    where
        T: UnwrapFrom<Artifact<'env>> + Copy,
    {
        pending_list
            .map(|pending| {
                *T::unwrap_from(
                    self.tasks[pending.raw_task_ref()]
                        .completed()
                        .as_ref()
                        .expect("artifact expected"),
                )
            })
            .collect()
    }

    pub fn demand_many_assoc<T, K>(
        &self,
        pending_list: impl Iterator<Item = (K, Pending<'env, T>)>,
    ) -> BoxedSlice<(K, T)>
    where
        T: UnwrapFrom<Artifact<'env>> + Copy,
        K: Copy,
    {
        pending_list
            .map(|(key, pending)| {
                (
                    key,
                    *T::unwrap_from(
                        self.tasks[pending.raw_task_ref()]
                            .completed()
                            .as_ref()
                            .expect("artifact expected"),
                    ),
                )
            })
            .collect()
    }

    pub fn expect_artifact(&self, task_ref: TaskRef<'env>) -> &Artifact<'env> {
        self.tasks[task_ref]
            .completed()
            .as_ref()
            .expect("artifact expected")
    }
}
