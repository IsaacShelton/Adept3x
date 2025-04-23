#[allow(unused_imports)]
use arena::{Arena, ArenaMap, Id, new_id_with_niche};
use arena::{Idx, IdxSpan};
use arrayvec::ArrayVec;
#[allow(unused_imports)]
use asg::PolyRecipe;
use ast_workspace::AstWorkspace;
use compiler::BuildOptions;
#[allow(unused_imports)]
use std::collections::{HashMap, VecDeque};
use std::{
    sync::{
        Condvar, Mutex, RwLock,
        atomic::{AtomicUsize, Ordering},
    },
    thread,
    time::Instant,
};

// This will be the driver for:
// - Building ASG Entries
// - Building IR Entries
// - Computing Sizes of IR Entries
// - Running Partially Completed IR

#[allow(dead_code)]
fn compile(_workspace: &AstWorkspace, options: &BuildOptions) {
    let num_threads = options.available_parallelism.get();
    let mut scheduler = Scheduler::new(num_threads);
    scheduler.start(|_, _| ());
}

#[derive(Debug)]
#[allow(dead_code)]
pub struct Job {
    content: &'static str,
}

new_id_with_niche!(JobId, u64);
pub type JobRef = Idx<JobId, Job>;

#[derive(Debug)]
pub struct Work {
    work: ArrayVec<JobRef, 128>,
}

#[derive(Debug)]
pub struct Scheduler {
    queue: Mutex<VecDeque<JobRef>>,
    jobs: RwLock<Arena<JobId, Job>>,
    num_active_threads: AtomicUsize,
    length: AtomicUsize,
    condvar: Condvar,
    done: Mutex<bool>,
    wakes: AtomicUsize,
    asks: AtomicUsize,
    total_did: AtomicUsize,
    available_parallelism: usize,
}

impl Scheduler {
    pub fn new(num_active_threads: usize) -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            jobs: RwLock::new(Arena::new()),
            num_active_threads: num_active_threads.into(),
            condvar: Condvar::new(),
            done: Mutex::new(false),
            length: AtomicUsize::new(0),
            wakes: AtomicUsize::new(0),
            asks: AtomicUsize::new(0),
            total_did: AtomicUsize::new(0),
            available_parallelism: num_active_threads,
        }
    }

    pub fn start<F: Fn(&Scheduler, JobRef) + Send + Sync>(&mut self, func: F) {
        thread::scope(|scope| {
            for _ in 0..self.num_active_threads.load(Ordering::SeqCst) {
                scope.spawn(|| {
                    while let Some(work) = self.ask_more_work() {
                        let count = work.work.len();
                        for job_ref in work.work {
                            (func)(self, job_ref);
                        }
                        self.total_did.fetch_add(count, Ordering::Relaxed);
                    }
                });
            }
        });
    }

    pub fn push(&self, job: Job) -> JobRef {
        let job_ref = self.jobs.write().unwrap().alloc(job);
        self.queue.lock().unwrap().push_back(job_ref);
        if self.length.fetch_add(1, Ordering::SeqCst)
            >= self.num_active_threads.load(Ordering::SeqCst)
        {
            self.condvar.notify_all();
        }
        job_ref
    }

    pub fn push_many(&self, jobs: impl IntoIterator<Item = Job>) -> IdxSpan<JobId, Job> {
        let job_refs = self.jobs.write().unwrap().alloc_many(jobs);
        self.queue.lock().unwrap().extend(job_refs);
        if self.length.fetch_add(1, Ordering::SeqCst)
            >= self.num_active_threads.load(Ordering::SeqCst)
        {
            self.condvar.notify_all();
        }
        job_refs
    }

    fn ask_more_work(&self) -> Option<Work> {
        loop {
            {
                self.asks.fetch_add(1, Ordering::Relaxed);
                let mut work = ArrayVec::new();
                {
                    let mut queue = self.queue.lock().unwrap();
                    let count = if queue.len() >= self.available_parallelism {
                        (queue.len() / self.available_parallelism).min(128)
                    } else {
                        queue.len().min(1)
                    };

                    for _ in 0..count {
                        work.push(queue.pop_back().unwrap());
                    }
                }

                if !work.is_empty() {
                    self.length.fetch_sub(work.len(), Ordering::SeqCst);
                    return Some(Work { work });
                }
            }

            // Wake everyone up if last one awake. Otherwise sleep.
            if self.num_active_threads.fetch_sub(1, Ordering::SeqCst) == 1 {
                *self.done.lock().unwrap() = true;
                self.condvar.notify_all();
                return None;
            }

            let mut done = self.done.lock().unwrap();
            loop {
                if *done {
                    return None;
                }

                done = self.condvar.wait(done).unwrap();

                // Should wake?
                if self.length.load(Ordering::SeqCst) != 0 {
                    self.num_active_threads.fetch_add(1, Ordering::SeqCst);
                    self.wakes.fetch_add(1, Ordering::Relaxed);
                    break;
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum AstSymbol {
    Func(ast::Func),
    Struct(ast::Struct),
    Enum(ast::Enum),
    Global(ast::Global),
    TypeAlias(ast::TypeAlias),
    ExprAlias(ast::ExprAlias),
    Trait(ast::Trait),
    Impl(ast::Impl),
}

pub fn main() {
    let earlier = Instant::now();
    let num_active_threads = 8;
    let mut scheduler = Scheduler::new(num_active_threads);
    let count = AtomicUsize::new(0);

    for _ in 0..10 {
        scheduler.push(Job { content: "Hello" });
    }

    scheduler.start(|scheduler, job_ref| {
        {
            let _job = &scheduler.jobs.read().unwrap()[job_ref];
            //println!("Doing - {}", job.content);
        }

        let i = count.fetch_add(1, Ordering::Relaxed);
        if i < 1_000_000 {
            scheduler.push_many([
                Job { content: "Hello" },
                Job { content: "World" },
                Job { content: "Bye" },
                Job { content: "Now" },
            ]);
        }
    });

    let took = Instant::now().duration_since(earlier);

    // Okay since synced
    println!("woke up {} times", scheduler.wakes.load(Ordering::Relaxed));
    println!("asked {} times", scheduler.asks.load(Ordering::Relaxed));
    println!(
        "total did {} tasks",
        scheduler.total_did.load(Ordering::Relaxed)
    );
    println!("took {:?}", took);
}
