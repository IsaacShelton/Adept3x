use crate::{Execution, Request, Task, TaskId, TaskRef, TaskState, Truth, WaitingCount, Worker};
use arena::Arena;
use crossbeam_deque::{Injector as InjectorQueue, Stealer};
use std::{
    num::NonZero,
    ops::DerefMut,
    sync::{
        RwLock,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

pub struct WorkerRef(pub usize);

pub struct MainExecutor<'env> {
    pub workers: Box<[Worker<'env>]>,
    pub executor: Executor<'env>,
}

pub struct Executor<'env> {
    pub injector: InjectorQueue<TaskRef<'env>>,
    pub truth: RwLock<Truth<'env>>,
    pub stealers: Box<[Stealer<TaskRef<'env>>]>,
    pub num_completed: AtomicUsize,
    pub num_scheduled: AtomicUsize,
    pub num_queued: AtomicUsize,
    pub num_cleared: AtomicUsize,
}

impl<'env> Executor<'env> {
    #[must_use]
    pub fn new(stealers: Box<[Stealer<TaskRef<'env>>]>) -> Self {
        Self {
            truth: RwLock::new(Truth::new()),
            injector: InjectorQueue::new(),
            stealers,
            num_scheduled: AtomicUsize::new(0),
            num_completed: AtomicUsize::new(0),
            num_queued: AtomicUsize::new(0),
            num_cleared: AtomicUsize::new(0),
        }
    }

    #[must_use]
    pub fn request(&self, request: impl Into<Request<'env>>) -> TaskRef<'env> {
        let request = request.into();
        let mut truth_guard = self.truth.write().unwrap();
        let truth = truth_guard.deref_mut();

        let tasks = &mut truth.tasks;
        let requests = &mut truth.requests;

        *requests.entry(request).or_insert_with_key(|request| {
            self.push_unique_into_tasks(tasks, &request.prereqs(), request.spawn_execution())
        })
    }

    #[must_use]
    pub fn push_unique(
        &self,
        suspend_on: &[TaskRef<'env>],
        execution: impl Into<Execution<'env>>,
    ) -> TaskRef<'env> {
        self.push_unique_into_tasks(
            &mut self.truth.write().unwrap().tasks,
            suspend_on,
            execution,
        )
    }

    #[must_use]
    pub fn push_unique_into_tasks(
        &self,
        tasks: &mut Arena<TaskId, Task<'env>>,
        suspend_on: &[TaskRef<'env>],
        execution: impl Into<Execution<'env>>,
    ) -> TaskRef<'env> {
        self.num_scheduled.fetch_add(1, Ordering::SeqCst);

        let mut wait_on = 0;

        let new_task_ref = {
            let new_task_ref = tasks.alloc(Task {
                state: TaskState::Suspended(execution.into(), WaitingCount(suspend_on.len())),
                dependents: vec![],
            });

            for dependent in suspend_on {
                if tasks[*dependent].state.completed().is_none() {
                    tasks[*dependent].dependents.push(new_task_ref);
                    wait_on += 1;
                }
            }

            new_task_ref
        };

        if wait_on == 0 {
            self.num_queued.fetch_add(1, Ordering::SeqCst);
            self.injector.push(new_task_ref);
        }

        new_task_ref
    }
}

#[derive(Debug)]
pub struct Executed<'env> {
    pub num_completed: usize,
    pub num_scheduled: usize,
    pub num_cleared: usize,
    pub num_queued: usize,
    pub truth: Truth<'env>,
}

impl<'env> MainExecutor<'env> {
    #[must_use]
    pub fn new(num_threads: NonZero<usize>) -> Self {
        let workers = (0..num_threads.get())
            .map(|worker_id| Worker::new(WorkerRef(worker_id)))
            .collect::<Box<_>>();

        let stealers = workers
            .iter()
            .map(|worker| worker.queue.stealer())
            .collect::<Box<_>>();

        Self {
            executor: Executor::new(stealers),
            workers,
        }
    }

    #[must_use]
    pub fn start(self) -> Executed<'env> {
        thread::scope(|scope| {
            for worker in self.workers.into_iter() {
                let executor = &self.executor;
                scope.spawn(move || worker.start(executor));
            }
        });

        Executed {
            num_completed: self.executor.num_completed.load(Ordering::Relaxed),
            num_scheduled: self.executor.num_scheduled.load(Ordering::Relaxed),
            num_cleared: self.executor.num_cleared.load(Ordering::Relaxed),
            num_queued: self.executor.num_queued.load(Ordering::Relaxed),
            truth: self.executor.truth.into_inner().unwrap(),
        }
    }

    #[must_use]
    pub fn push(
        &self,
        suspend_on: &[TaskRef<'env>],
        execution: impl Into<Execution<'env>>,
    ) -> TaskRef<'env> {
        self.executor.push_unique(suspend_on, execution)
    }
}
