/*
    =====================  components/job/src/pending.rs  =====================
    A zero-cost type-safe wrapper `Pending<'env, T>` around `TaskRef<'env>`.

    Used for easily extracting the output of the wrapped task.
    ---------------------------------------------------------------------------
*/

use crate::{Artifact, TaskRef, UnwrapFrom};
use std::marker::PhantomData;

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
