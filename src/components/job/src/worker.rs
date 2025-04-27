use crate::{
    Artifact, Execute, Execution, Executor, Progression, TaskRef, TaskState, WaitingCount,
    WorkerRef,
};
use std::mem;

pub struct Worker {
    pub done: bool,
    pub batch: Vec<TaskRef>,
}

impl Worker {
    pub fn new() -> Self {
        Worker {
            done: false,
            batch: Vec::new(),
        }
    }

    pub fn start(worker_ref: WorkerRef, executor: &Executor) {
        let mut guard = executor.workers[worker_ref].0.lock().unwrap();

        loop {
            let (task_ref, execution) = loop {
                if let Some(has) = guard.next_task(executor) {
                    break has;
                }

                if guard.done {
                    return;
                }

                guard = executor.workers[worker_ref].1.wait(guard).unwrap();
            };

            match execution.execute(executor).progression() {
                Progression::Complete(artifact) => {
                    guard.complete(executor, task_ref, artifact);
                }
                Progression::Suspend(waiting, execution) => {
                    guard.suspend(executor, task_ref, waiting, execution);
                }
            }
        }
    }

    fn next_task(&self, executor: &Executor) -> Option<(TaskRef, Execution)> {
        let mut truth = executor.truth.write().unwrap();
        let task_ref = truth.queue.pop_front()?;

        let (execution, waiting_count) =
            mem::replace(&mut truth.tasks[task_ref].state, TaskState::Running).unwrap_suspended();

        assert_eq!(waiting_count.0, 0);
        Some((task_ref, execution))
    }

    fn complete(&self, executor: &Executor, task_ref: TaskRef, artifact: Artifact) {
        let truth = &mut executor.truth.write().unwrap();

        let dependents = {
            let task = &mut truth.tasks[task_ref];
            task.state = TaskState::Completed(artifact);
            mem::take(&mut task.dependents)
        };

        for dependent in dependents {
            if let TaskState::Suspended(_, waiting_count) = &mut truth.tasks[dependent].state {
                if waiting_count.decrement() {
                    truth.queue.push_back(dependent);
                }
            }
        }
    }

    fn suspend(
        &self,
        executor: &Executor,
        task_ref: TaskRef,
        waiting: Vec<TaskRef>,
        execution: Execution,
    ) {
        let truth = &mut executor.truth.write().unwrap();
        let task = &mut truth.tasks[task_ref];
        task.state = TaskState::Suspended(execution, WaitingCount(waiting.len()));

        for dependent in &waiting {
            truth.tasks[*dependent].dependents.push(task_ref);
        }

        if waiting.len() == 0 {
            truth.queue.push_back(task_ref);
        }
    }
}
