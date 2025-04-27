use crate::{Execution, Task, TaskRef, TaskState, Truth, WaitingCount, Worker};
use arena::{Arena, Idx, IdxSpan, new_id_with_niche};
use std::{
    sync::{
        Condvar, Mutex, MutexGuard, RwLock,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
};

new_id_with_niche!(WorkerId, u8);
pub type WorkerRef = Idx<WorkerId, Worker>;

pub struct Executor {
    pub truth: RwLock<Truth>,
    pub workers: Arena<WorkerId, Worker>,
    pub worker_refs: IdxSpan<WorkerId, Worker>,
    pub workers_alive: AtomicUsize,
    pub done: Mutex<bool>,
    pub condvar: Condvar,
}

impl Executor {
    pub fn new(num_threads: usize) -> Self {
        let mut workers = Arena::new();
        let worker_refs =
            workers.alloc_many(std::iter::from_fn(|| Some(Worker::new())).take(num_threads));

        Self {
            truth: RwLock::new(Truth::new()),
            workers,
            worker_refs,
            workers_alive: AtomicUsize::new(num_threads),
            done: Mutex::new(false),
            condvar: Condvar::new(),
        }
    }
}

impl Executor {
    pub fn start(&self) {
        thread::scope(|scope| {
            for worker_ref in self.worker_refs.iter() {
                scope.spawn(move || Worker::start(worker_ref, self));
            }
        });
    }

    fn is_all_done<'a>(&self) -> bool {
        let is_last_worker = self.workers_alive.fetch_sub(1, Ordering::SeqCst) == 1;

        if is_last_worker {
            *self.done.lock().unwrap() = true;
            self.condvar.notify_all();
            return true;
        }

        let mut all_done = self.done.lock().unwrap();

        loop {
            if *all_done {
                return true;
            }

            if !self.truth.read().unwrap().queue.is_empty() {
                self.workers_alive.fetch_add(1, Ordering::SeqCst);
                return false;
            }

            all_done = self.condvar.wait(all_done).unwrap();
        }
    }

    pub fn push(&self, execution: impl Into<Execution>) -> TaskRef {
        let mut truth = self.truth.write().unwrap();

        let task_ref = truth.tasks.alloc(Task {
            state: TaskState::Suspended(execution.into(), WaitingCount::default()),
            dependents: vec![],
        });

        truth.queue.push_back(task_ref);

        // TODO: Improve this, and use separate queue for each worker
        for (_, (_, condvar)) in self.workers.iter() {
            condvar.notify_one();
        }

        task_ref
    }
}
