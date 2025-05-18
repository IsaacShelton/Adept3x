use crate::{
    Artifact, Continuation, Execution, Executor, RawExecutable, SuspendCondition, TaskRef,
    TaskState, WaitingCount, WorkerRef,
};
use crossbeam_deque::{Stealer, Worker as WorkerQueue};
use std::{iter, mem, sync::atomic::Ordering};

pub struct Worker<'env> {
    pub worker_ref: WorkerRef,
    pub queue: WorkerQueue<TaskRef<'env>>,
}

impl<'env> Worker<'env> {
    #[must_use]
    pub fn new(worker_ref: WorkerRef) -> Self {
        Worker {
            worker_ref,
            queue: WorkerQueue::new_lifo(),
        }
    }

    pub fn start(&self, executor: &Executor<'env>) {
        loop {
            if let Some(task_ref) = self.find_task(executor, &executor.stealers) {
                let execution = {
                    mem::replace(
                        &mut executor.truth.write().unwrap().tasks[task_ref].state,
                        TaskState::Running,
                    )
                    .unwrap_get_execution()
                };

                match execution.execute_raw(executor) {
                    Ok(artifact) => {
                        executor.num_completed.fetch_add(1, Ordering::SeqCst);
                        self.complete(executor, task_ref, artifact);
                    }
                    Err(Continuation::Suspend(waiting, execution)) => {
                        self.suspend(executor, task_ref, waiting, execution);
                    }
                    Err(Continuation::Error(error)) => {
                        eprintln!("error: {}", error);
                    }
                }

                executor.num_cleared.fetch_add(1, Ordering::SeqCst);
            } else if executor.num_cleared.load(Ordering::SeqCst)
                == executor.num_queued.load(Ordering::SeqCst)
            {
                break;
            }
        }
    }

    fn find_task(
        &self,
        executor: &Executor<'env>,
        stealers: &[Stealer<TaskRef<'env>>],
    ) -> Option<TaskRef<'env>> {
        // Pop a task from the local queue, if not empty.
        self.queue.pop().or_else(|| {
            // Otherwise, we need to look for a task elsewhere.
            iter::repeat_with(|| {
                // Try stealing a batch of tasks from the global queue.
                executor
                    .injector
                    .steal_batch_and_pop(&self.queue)
                    // Or try stealing a task from one of the other threads.
                    .or_else(|| stealers.iter().map(|s| s.steal()).collect())
            })
            // Loop while no task was stolen and any steal operation needs to be retried.
            .find(|s| !s.is_retry())
            // Extract the stolen task, if there is one.
            .and_then(|s| s.success())
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
            if let TaskState::Suspended(_, condition) = &mut truth.tasks[dependent].state {
                match condition {
                    SuspendCondition::All(waiting_count) => {
                        if waiting_count.decrement() {
                            executor.num_queued.fetch_add(1, Ordering::SeqCst);
                            self.queue.push(dependent);
                        }
                    }
                    SuspendCondition::Any(of) => {
                        if of.contains(&task_ref) {
                            of.clear();
                            executor.num_queued.fetch_add(1, Ordering::SeqCst);
                            self.queue.push(dependent);
                        }
                    }
                }
            }
        }
    }

    fn suspend(
        &self,
        executor: &Executor<'env>,
        task_ref: TaskRef<'env>,
        waiting: Vec<TaskRef<'env>>,
        execution: Execution<'env>,
    ) {
        let mut wait_on = 0;

        {
            let truth = &mut executor.truth.write().unwrap();

            for dependent in &waiting {
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
            self.queue.push(task_ref);
        }
    }
}
