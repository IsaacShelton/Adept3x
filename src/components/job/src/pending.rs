/*
    =====================  components/job/src/pending.rs  =====================
    A zero-cost type-safe wrapper `Pending<'env, T>` around `TaskRef<'env>`.

    Used for easily extracting the output of the wrapped task.
    ---------------------------------------------------------------------------
*/

use crate::{Artifact, TaskRef, UnwrapFrom};
use std::{hash::Hash, marker::PhantomData};

pub type PendingMany<'env, T> = Box<[Pending<'env, T>]>;

pub type Suspend<'env, T> = Option<Pending<'env, T>>;
pub type SuspendMany<'env, T> = Option<PendingMany<'env, T>>;

#[derive(Debug, PartialEq, Eq)]
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

impl<'env, T> Hash for Pending<'env, T>
where
    T: UnwrapFrom<Artifact<'env>>,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.task_ref.hash(state)
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

impl<'env, T> From<Pending<'env, T>> for TaskRef<'env>
where
    T: UnwrapFrom<Artifact<'env>>,
{
    fn from(value: Pending<'env, T>) -> Self {
        value.raw_task_ref()
    }
}

impl<'env, T> From<&Pending<'env, T>> for TaskRef<'env>
where
    T: UnwrapFrom<Artifact<'env>>,
{
    fn from(value: &Pending<'env, T>) -> Self {
        value.raw_task_ref()
    }
}

impl<'env, T> IntoIterator for Pending<'env, T>
where
    T: UnwrapFrom<Artifact<'env>>,
{
    type Item = TaskRef<'env>;
    type IntoIter = std::iter::Once<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.into())
    }
}
