use crate::{Artifact, TaskState, UnwrapFrom};
use arena::{Idx, new_id_with_niche};
use std::marker::PhantomData;

new_id_with_niche!(TaskId, u64);
pub type TaskRef<'env> = Idx<TaskId, Task<'env>>;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Pending<'env, T>
where
    T: UnwrapFrom<Artifact<'env>>,
{
    task_ref: TaskRef<'env>,
    _phantom: PhantomData<T>,
}

// NOTE: We have to manually derive Copy + Clone to prevent T: Copy requirement - https://github.com/rust-lang/rust/issues/26925
impl<'env, T> Copy for Pending<'env, T> where T: UnwrapFrom<Artifact<'env>> {}
impl<'env, T> Clone for Pending<'env, T>
where
    T: UnwrapFrom<Artifact<'env>>,
{
    fn clone(&self) -> Self {
        *self
    }
}

impl<'env, T> Pending<'env, T>
where
    T: UnwrapFrom<Artifact<'env>>,
{
    pub fn new_unchecked(task_ref: TaskRef<'env>) -> Self {
        Self {
            task_ref,
            _phantom: PhantomData,
        }
    }

    pub fn raw_task_ref(&self) -> TaskRef<'env> {
        self.task_ref
    }
}

pub trait Extract {
    type Extractee<'a>;
    fn extract<'a>(artifact: &'a Artifact) -> Self::Extractee<'a>;
}

#[derive(Debug)]
pub struct Task<'env> {
    pub state: TaskState<'env>,
    pub dependents: Vec<TaskRef<'env>>,
}

impl<'env> Task<'env> {
    pub fn completed(&self) -> Option<&Artifact<'env>> {
        self.state.completed()
    }
}
