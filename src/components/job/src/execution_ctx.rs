use crate::{BumpAllocator, TaskRef, io::IoResponse};
use derive_more::Deref;

#[derive(Deref)]
pub struct ExecutionCtx<'env> {
    #[deref]
    allocator: &'env bumpalo::Bump,
    suspend_on: Vec<TaskRef<'env>>,
    self_task: Option<TaskRef<'env>>,
    io_response: Option<IoResponse>,
}

impl<'env> ExecutionCtx<'env> {
    pub fn new(allocator: &'env BumpAllocator) -> Self {
        Self {
            allocator,
            suspend_on: Vec::with_capacity(32),
            self_task: None,
            io_response: None,
        }
    }

    pub fn io_response(&mut self) -> Option<IoResponse> {
        self.io_response.take()
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

    pub fn self_task(&mut self) -> TaskRef<'env> {
        self.self_task.expect("task to be set")
    }

    pub fn prepare_for_task(
        &mut self,
        new_self_task: TaskRef<'env>,
        io_response: Option<IoResponse>,
    ) {
        self.self_task = Some(new_self_task);
        self.io_response = io_response;
    }
}
