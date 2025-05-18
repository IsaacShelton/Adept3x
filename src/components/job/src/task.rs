/*
    ======================  components/job/src/task.rs  =======================
    Defines what a task is in the job system.
    ---------------------------------------------------------------------------
*/

use crate::{Artifact, TaskState};
use arena::{Idx, new_id_with_niche};

new_id_with_niche!(TaskId, u64);
pub type TaskRef<'env> = Idx<TaskId, Task<'env>>;

#[derive(Debug)]
pub struct Task<'env> {
    pub state: TaskState<'env>,
    pub dependents: Vec<TaskRef<'env>>,
}

impl<'env> Task<'env> {
    pub fn completed(&self) -> Option<&Artifact<'env>> {
        self.state.completed()
    }
}
