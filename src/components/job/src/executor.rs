use crate::{Execution, Task, TaskRef, TaskState, Truth, WaitingCount, Worker};
use crossbeam_deque::{Injector as InjectorQueue, Stealer};
use std::{
    sync::{
        RwLock,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

pub struct WorkerRef(pub usize);

pub struct MainExecutor {
    pub workers: Box<[Worker]>,
    pub executor: Executor,
}

pub struct Executor {
    pub injector: InjectorQueue<TaskRef>,
    pub truth: RwLock<Truth>,
    pub stealers: Box<[Stealer<TaskRef>]>,
    pub num_completed: AtomicUsize,
    pub num_scheduled: AtomicUsize,
    pub num_queued: AtomicUsize,
    pub num_cleared: AtomicUsize,
}

impl Executor {
    pub fn new(stealers: Box<[Stealer<TaskRef>]>) -> Self {
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

    pub fn push(&self, execution: impl Into<Execution>) -> TaskRef {
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

#[derive(Clone, Debug)]
pub struct MainExecutorStats {
    pub num_completed: usize,
    pub num_scheduled: usize,
    pub num_cleared: usize,
    pub num_queued: usize,
}

impl MainExecutor {
    pub fn new(num_threads: usize) -> Self {
        let workers = (0..num_threads)
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

    pub fn start(self) -> MainExecutorStats {
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
        }
    }

    pub fn push(&self, execution: impl Into<Execution>) -> TaskRef {
        self.executor.push(execution)
    }
}
