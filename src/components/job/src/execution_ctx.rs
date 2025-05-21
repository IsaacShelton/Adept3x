use crate::{BumpAllocator, TaskRef};
use derive_more::Deref;

#[derive(Deref)]
pub struct ExecutionCtx<'env> {
    #[deref]
    allocator: &'env bumpalo::Bump,
    suspend_on: Vec<TaskRef<'env>>,
}

impl<'env> ExecutionCtx<'env> {
    pub fn new(allocator: &'env BumpAllocator) -> Self {
        Self {
            allocator,
            suspend_on: Vec::with_capacity(32),
        }
    }

    pub fn suspend_on(&mut self, tasks: impl IntoIterator<Item = impl Into<TaskRef<'env>>>) {
        self.suspend_on
            .extend(tasks.into_iter().map(|task| task.into()));
    }

    pub fn waiting_on(&self) -> &[TaskRef<'env>] {
        &self.suspend_on
    }

    pub fn reset_waiting_on(&mut self) {
        self.suspend_on.clear();
    }
}
