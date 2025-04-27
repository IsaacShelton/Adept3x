use crate::{Task, TaskId, TaskRef};
use arena::Arena;
use std::collections::VecDeque;

pub struct Truth {
    pub tasks: Arena<TaskId, Task>,
    pub queue: VecDeque<TaskRef>,
}

impl Truth {
    pub fn new() -> Self {
        Self {
            tasks: Arena::new(),
            queue: VecDeque::new(),
        }
    }
}
