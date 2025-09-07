/*
    ==================  components/job/src/main_executor.rs  ==================
    The main overarching executor that encompasses the entire job system.

    It's used to hold state that workers aren't allowed to access.

    Instead, each worker can only access data via `Executor`, never `MainExecutor`.
    ---------------------------------------------------------------------------
*/

use crate::{
    BumpAllocatorPool, Executable, Execution, Executor, Pending, SuspendCondition, TaskId, TaskRef,
    TaskState, TopN, Truth, Worker, WorkerRef, io::IoResponse,
};
use diagnostics::{Diagnostics, ErrorDiagnostic};
use source_files::SourceFiles;
use std::{
    sync::{atomic::Ordering, mpsc},
    thread,
    time::Duration,
};
use threadpool::ThreadPool;

pub struct MainExecutor<'env> {
    pub executor: Executor<'env>,
    pub io_rx: mpsc::Receiver<(TaskId, IoResponse)>,
}

impl<'env> MainExecutor<'env> {
    #[must_use]
    pub fn new(io_thread_pool: &'env ThreadPool, diagnostics: &'env Diagnostics<'env>) -> Self {
        let (io_tx, io_rx) = mpsc::channel::<(TaskId, IoResponse)>();

        Self {
            executor: Executor::new(io_thread_pool, io_tx, diagnostics),
            io_rx,
        }
    }

    #[must_use]
    pub fn start(
        self,
        source_files: &SourceFiles,
        allocator_pool: &'env mut BumpAllocatorPool,
    ) -> MainExecutorStats<'env> {
        let workers = (0..allocator_pool.len().get())
            .map(|worker_id| Worker::new(WorkerRef(worker_id)))
            .collect::<Box<_>>();

        let stealers = workers
            .iter()
            .map(|worker| worker.local_queue.stealer())
            .collect::<Box<_>>();

        let max_top_errors = 10;

        let mut errors = thread::scope(|scope| {
            let mut top_n_trackers = Vec::with_capacity(workers.len());

            for (worker, allocator) in workers
                .into_iter()
                .zip(allocator_pool.allocators.iter_mut())
            {
                let executor = &self.executor;
                let stealers = &stealers;
                top_n_trackers.push(scope.spawn(move || {
                    worker.start(source_files, max_top_errors, executor, allocator, stealers)
                }));
            }

            loop {
                match self.io_rx.recv_timeout(Duration::from_millis(1)) {
                    Ok((task_id, io_response)) => {
                        let task_ref = unsafe { TaskRef::from_raw(task_id) };

                        match &mut self.executor.truth.write().unwrap().tasks[task_ref].state {
                            TaskState::Suspended(_, condition @ SuspendCondition::PendingIo) => {
                                *condition = SuspendCondition::WakeFromIo(io_response)
                            }
                            _ => panic!(
                                "Cannot receive completed IO for task that's not pending IO!"
                            ),
                        };

                        self.executor.injector.push(task_ref);
                        self.executor.num_cleared.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(_timeout) => {
                        if self.executor.num_cleared.load(Ordering::SeqCst)
                            == self.executor.num_queued.load(Ordering::SeqCst)
                        {
                            break;
                        }
                    }
                }
            }

            TopN::from_iter(
                max_top_errors,
                top_n_trackers
                    .into_iter()
                    .flat_map(|tracker| tracker.join().unwrap().into_iter()),
                |a, b| a.cmp_with(b, source_files),
            )
        });

        self.executor.io_thread_pool.join();
        self.executor
            .pending_searches
            .report_errors(&mut errors, source_files);

        MainExecutorStats {
            num_completed: self.executor.num_completed.load(Ordering::Relaxed),
            num_scheduled: self.executor.num_scheduled.load(Ordering::Relaxed),
            num_cleared: self.executor.num_cleared.load(Ordering::Relaxed),
            num_queued: self.executor.num_queued.load(Ordering::Relaxed),
            truth: self.executor.truth.into_inner().unwrap(),
            errors,
            num_unresolved_symbol_references: self
                .executor
                .pending_searches
                .num_unresolved_symbol_references(),
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
    pub errors: TopN<ErrorDiagnostic>,
    pub num_unresolved_symbol_references: usize,
}
