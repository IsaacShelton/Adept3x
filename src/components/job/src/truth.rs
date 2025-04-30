use crate::{Task, TaskId};
use arena::Arena;

pub struct Truth {
    pub tasks: Arena<TaskId, Task>,
}

impl Truth {
    pub fn new() -> Self {
        Self {
            tasks: Arena::new(),
        }
    }
}
