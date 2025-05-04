use crate::TaskState;
use arena::{Idx, new_id_with_niche};

new_id_with_niche!(TaskId, u64);
pub type TaskRef<'outside> = Idx<TaskId, Task<'outside>>;

#[derive(Debug)]
pub struct Task<'outside> {
    pub state: TaskState<'outside>,
    pub dependents: Vec<TaskRef<'outside>>,
}
