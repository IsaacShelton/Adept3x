/*
    ==================  components/job/src/main_executor.rs  ==================
    The main overarching executor that encompasses the entire job system.

    It's used to hold state that workers aren't allowed to access.

    Instead, each worker can only access data via `Executor`, never `MainExecutor`.
    ---------------------------------------------------------------------------
*/

use crate::{Executable, Execution, Executor, Pending, TaskRef, Truth, Worker, WorkerRef};
use std::{num::NonZero, sync::atomic::Ordering, thread};

pub struct MainExecutor<'env> {
    pub workers: Box<[Worker<'env>]>,
    pub executor: Executor<'env>,
}

impl<'env> MainExecutor<'env> {
    #[must_use]
    pub fn new(num_threads: NonZero<usize>) -> Self {
        let workers = (0..num_threads.get())
            .map(|worker_id| Worker::new(WorkerRef(worker_id)))
            .collect::<Box<_>>();

        let stealers = workers
            .iter()
            .map(|worker| worker.local_queue.stealer())
            .collect::<Box<_>>();

        Self {
            executor: Executor::new(stealers),
            workers,
        }
    }

    #[must_use]
    pub fn start(self) -> MainExecutorStats<'env> {
        thread::scope(|scope| {
            for worker in self.workers.into_iter() {
                let executor = &self.executor;
                scope.spawn(move || worker.start(executor));
            }
        });

        MainExecutorStats {
            num_completed: self.executor.num_completed.load(Ordering::Relaxed),
            num_scheduled: self.executor.num_scheduled.load(Ordering::Relaxed),
            num_cleared: self.executor.num_cleared.load(Ordering::Relaxed),
            num_queued: self.executor.num_queued.load(Ordering::Relaxed),
            truth: self.executor.truth.into_inner().unwrap(),
        }
    }

    #[must_use]
    pub fn spawn<E>(
        &self,
        suspend_on: &[TaskRef<'env>],
        execution: E,
    ) -> Pending<'env, <E as Executable<'env>>::Output>
    where
        E: Into<Execution<'env>> + Executable<'env>,
    {
        Pending::new_unchecked(self.executor.push_unique(suspend_on, execution))
    }
}

#[derive(Debug)]
pub struct MainExecutorStats<'env> {
    pub num_completed: usize,
    pub num_scheduled: usize,
    pub num_cleared: usize,
    pub num_queued: usize,
    pub truth: Truth<'env>,
}
