use crate::TaskState;
use arena::{Idx, new_id_with_niche};

new_id_with_niche!(TaskId, u64);
pub type TaskRef<'env> = Idx<TaskId, Task<'env>>;

#[derive(Debug)]
pub struct Task<'env> {
    pub state: TaskState<'env>,
    pub dependents: Vec<TaskRef<'env>>,
}
