use crate::{Execution, Task, TaskRef, TaskState, Truth, WaitingCount, Worker};
use crossbeam_deque::{Injector as InjectorQueue, Stealer};
use std::{
    num::NonZero,
    sync::{
        RwLock,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

pub struct WorkerRef(pub usize);

pub struct MainExecutor<'outside> {
    pub workers: Box<[Worker<'outside>]>,
    pub executor: Executor<'outside>,
}

pub struct Executor<'outside> {
    pub injector: InjectorQueue<TaskRef<'outside>>,
    pub truth: RwLock<Truth<'outside>>,
    pub stealers: Box<[Stealer<TaskRef<'outside>>]>,
    pub num_completed: AtomicUsize,
    pub num_scheduled: AtomicUsize,
    pub num_queued: AtomicUsize,
    pub num_cleared: AtomicUsize,
}

impl<'outside> Executor<'outside> {
    pub fn new(stealers: Box<[Stealer<TaskRef<'outside>>]>) -> Self {
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

    pub fn push(&self, execution: impl Into<Execution<'outside>>) -> TaskRef<'outside> {
        self.num_scheduled.fetch_add(1, Ordering::SeqCst);
        let task_ref = {
            let mut truth = self.truth.write().unwrap();

            truth.tasks.alloc(Task {
                state: TaskState::Suspended(execution.into(), WaitingCount::default()),
                dependents: vec![],
            })
        };

        self.num_queued.fetch_add(1, Ordering::SeqCst);
        self.injector.push(task_ref);
        task_ref
    }
}

#[derive(Debug)]
pub struct Executed<'outside> {
    pub num_completed: usize,
    pub num_scheduled: usize,
    pub num_cleared: usize,
    pub num_queued: usize,
    pub truth: Truth<'outside>,
}

impl<'outside> MainExecutor<'outside> {
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

    pub fn start(self) -> Executed<'outside> {
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

    pub fn push(&self, execution: impl Into<Execution<'outside>>) -> TaskRef<'outside> {
        self.executor.push(execution)
    }
}
