use crate::TaskState;
use arena::{Idx, new_id_with_niche};

new_id_with_niche!(TaskId, u64);
pub type TaskRef = Idx<TaskId, Task>;

#[derive(Debug)]
pub struct Task {
    pub state: TaskState,
    pub dependents: Vec<TaskRef>,
}
