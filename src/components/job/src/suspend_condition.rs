/*
    ================  components/job/src/suspend_condition.rs  ================
    Defines how tasks are allowed to suspend.
    ---------------------------------------------------------------------------
*/

use crate::{TaskRef, WaitingCount};
use smallvec::SmallVec;

#[derive(Debug)]
pub enum SuspendCondition<'env> {
    /// Wait for all dependent tasks to complete before waking up
    All(WaitingCount),

    /// Wait for any of these specified dependent tasks to complete before waking up
    Any(SmallVec<[TaskRef<'env>; 2]>),

    /// Pending IO
    PendingIo,
}
