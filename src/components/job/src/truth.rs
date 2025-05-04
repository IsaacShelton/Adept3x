use crate::{Task, TaskId};
use arena::Arena;

#[derive(Debug)]
pub struct Truth<'outside> {
    pub tasks: Arena<TaskId, Task<'outside>>,
}

impl Truth<'_> {
    pub fn new() -> Self {
        Self {
            tasks: Arena::new(),
        }
    }
}
