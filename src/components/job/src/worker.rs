use crate::{
    Artifact, BumpAllocator, Continuation, Execution, ExecutionCtx, Executor, RawExecutable,
    SuspendCondition, TaskRef, TaskState, TopN, WaitingCount,
    io::{IoRequest, IoResponse},
};
use crossbeam_deque::{Stealer, Worker as WorkerQueue};
use diagnostics::ErrorDiagnostic;
use source_files::{Source, SourceFiles};
use std::{
    iter::{self},
    mem,
    panic::{self, AssertUnwindSafe},
    sync::atomic::Ordering,
};

#[derive(Copy, Clone, Debug)]
pub struct WorkerRef(pub usize);

pub struct Worker<'env> {
    pub worker_ref: WorkerRef,
    pub local_queue: WorkerQueue<TaskRef<'env>>,
}

impl<'env> Worker<'env> {
    #[must_use]
    pub fn new(worker_ref: WorkerRef) -> Self {
        Worker {
            worker_ref,
            local_queue: WorkerQueue::new_lifo(),
        }
    }

    pub fn start(
        &self,
        source_files: &SourceFiles,
        max_top_errors: usize,
        executor: &Executor<'env>,
        allocator: &'env BumpAllocator,
        stealers: &[Stealer<TaskRef<'env>>],
    ) -> TopN<ErrorDiagnostic> {
        let mut ctx = ExecutionCtx::new(allocator);
        let mut top_n_errors = TopN::new(max_top_errors);

        loop {
            if let Some((task_ref, execution, io_response)) = self.find_task(executor, stealers) {
                ctx.prepare_for_task(task_ref, io_response);

                let result = panic::catch_unwind(AssertUnwindSafe(|| {
                    execution.execute_raw(executor, &mut ctx)
                }));

                match result {
                    Ok(Ok(artifact)) => {
                        executor.num_completed.fetch_add(1, Ordering::SeqCst);
                        self.complete(executor, task_ref, artifact);
                        executor.num_cleared.fetch_add(1, Ordering::SeqCst);
                    }
                    Ok(Err(Continuation::Suspend(execution))) => {
                        self.suspend(executor, task_ref, ctx.waiting_on(), execution);
                        ctx.reset_waiting_on();
                        executor.num_cleared.fetch_add(1, Ordering::SeqCst);
                    }
                    Ok(Err(Continuation::PendingSearch(
                        execution,
                        graph_ref,
                        searched_version,
                        search,
                    ))) => {
                        executor.truth.write().unwrap().tasks[task_ref].state =
                            TaskState::Suspended(execution, SuspendCondition::PendingSymbol);

                        if let Err(already_ready) = executor.pending_searches.get_or_default(
                            graph_ref,
                            |pending_search_map| {
                                pending_search_map.suspend_on(
                                    searched_version,
                                    search.name(),
                                    task_ref,
                                )
                            },
                        ) {
                            executor.num_queued.fetch_add(1, Ordering::SeqCst);
                            self.local_queue.push(already_ready);
                        }

                        executor.num_cleared.fetch_add(1, Ordering::SeqCst);
                    }
                    Ok(Err(Continuation::RequestIo(execution, io_request))) => {
                        executor.truth.write().unwrap().tasks[task_ref].state =
                            TaskState::Suspended(execution, SuspendCondition::PendingIo);

                        executor.num_queued.fetch_add(1, Ordering::SeqCst);

                        let task_id = task_ref.into_raw();
                        let io_tx = executor.io_tx.clone();
                        executor.io_thread_pool.execute(move || match io_request {
                            IoRequest::ReadFile(path) => {
                                io_tx
                                    .send((
                                        task_id,
                                        IoResponse {
                                            payload: std::fs::read_to_string(path)
                                                .map_err(|e| e.to_string()),
                                        },
                                    ))
                                    .expect("failed to send io response");
                            }
                        });
                    }
                    Ok(Err(Continuation::Error(error))) => {
                        top_n_errors.push(error, |a, b| a.cmp_with(b, source_files));
                        executor.num_cleared.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(_) => {
                        top_n_errors.push(
                            ErrorDiagnostic::new(
                                format!("Internal Compiler Error! Task paniced during execution!"),
                                Source::internal(),
                            ),
                            |a, b| a.cmp_with(b, source_files),
                        );
                        executor.num_cleared.fetch_add(1, Ordering::SeqCst);
                    }
                }
            } else if executor.num_cleared.load(Ordering::SeqCst)
                == executor.num_queued.load(Ordering::SeqCst)
            {
                break;
            }
        }

        top_n_errors
    }

    #[must_use]
    fn find_task(
        &self,
        executor: &Executor<'env>,
        stealers: &[Stealer<TaskRef<'env>>],
    ) -> Option<(TaskRef<'env>, Execution<'env>, Option<IoResponse>)> {
        // Try to find task (in the order of local queue, global queue, other worker queues)
        let task_ref = self.local_queue.pop().or_else(|| {
            iter::repeat_with(|| {
                executor
                    .injector
                    .steal_batch_and_pop(&self.local_queue)
                    .or_else(|| stealers.iter().map(|s| s.steal()).collect())
            })
            .find(|s| !s.is_retry())
            .and_then(|s| s.success())
        });

        let mut truth = executor.truth.write().unwrap();

        // If found a task, extract its execution that needs to be run
        task_ref.map(|task_ref| {
            let state = mem::take(&mut truth.tasks[task_ref].state);

            match state {
                TaskState::Suspended(execution, suspend_condition) => {
                    (task_ref, execution, suspend_condition.to_io_response())
                }
                _ => panic!("Cannot run task that isn't suspended!"),
            }
        })
    }

    fn complete(
        &self,
        executor: &Executor<'env>,
        task_ref: TaskRef<'env>,
        artifact: Artifact<'env>,
    ) {
        let truth = &mut executor.truth.write().unwrap();

        let dependents = {
            let task = &mut truth.tasks[task_ref];
            task.state = TaskState::Completed(artifact);
            mem::take(&mut task.dependents)
        };

        for dependent in dependents {
            let TaskState::Suspended(_, condition) = &mut truth.tasks[dependent].state else {
                continue;
            };

            match condition {
                SuspendCondition::All(waiting_count) => {
                    if waiting_count.decrement() {
                        executor.num_queued.fetch_add(1, Ordering::SeqCst);
                        self.local_queue.push(dependent);
                    }
                }
                SuspendCondition::Any(of) => {
                    if of.contains(&task_ref) {
                        of.clear();
                        executor.num_queued.fetch_add(1, Ordering::SeqCst);
                        self.local_queue.push(dependent);
                    }
                }
                SuspendCondition::PendingIo
                | SuspendCondition::WakeFromIo(..)
                | SuspendCondition::PendingSymbol => {
                    unreachable!("cannot wake dependent task not in a dependent suspend condition")
                }
            }
        }
    }

    fn suspend(
        &self,
        executor: &Executor<'env>,
        task_ref: TaskRef<'env>,
        waiting: &[TaskRef<'env>],
        execution: Execution<'env>,
    ) {
        let mut wait_on = 0;

        {
            let truth = &mut executor.truth.write().unwrap();

            for dependent in waiting {
                if truth.tasks[*dependent].state.completed().is_none() {
                    truth.tasks[*dependent].dependents.push(task_ref);
                    wait_on += 1;
                }
            }

            truth.tasks[task_ref].state =
                TaskState::Suspended(execution, SuspendCondition::All(WaitingCount(wait_on)));
        }

        if wait_on == 0 {
            executor.num_queued.fetch_add(1, Ordering::SeqCst);
            self.local_queue.push(task_ref);
        }
    }
}
