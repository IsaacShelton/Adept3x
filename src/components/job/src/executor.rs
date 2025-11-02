/*
    ====================  components/job/src/executor.rs  =====================
    Defines what a worker sees as it's `Executor`.

    This only contains parts of the executor that workers are allowed to see.

    Data that must be kept inaccessible to workers is instead kept in `MainExecutor`.
    ---------------------------------------------------------------------------
*/

use crate::{
    Artifact, Executable, Execution, ModuleGraphPendingSearchMap, Pending, Request, Spawnable,
    SuspendCondition, SuspendMany, SuspendManyAssoc, Task, TaskId, TaskRef, TaskState, Truth,
    UnwrapFrom, WaitingCount, io::IoResponse, module_graph::ModuleGraphRef,
};
use arena::Arena;
use crossbeam_deque::Injector as InjectorQueue;
use diagnostics::Diagnostics;
use std::{
    ops::DerefMut,
    sync::{
        RwLock,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
};
use std_ext::BoxedSlice;
use threadpool::ThreadPool;

pub struct Executor<'env> {
    // Global work-stealing queue injector
    pub injector: InjectorQueue<TaskRef<'env>>,

    // Task Data
    pub truth: RwLock<Truth<'env>>,

    // Count for tasks
    pub num_completed: AtomicUsize,
    pub num_scheduled: AtomicUsize,

    // Count for task executions (including suspend/resumes)
    pub num_queued: AtomicUsize,
    pub num_cleared: AtomicUsize,

    // IO Thread Pool
    pub io_thread_pool: &'env ThreadPool,
    pub io_tx: mpsc::Sender<(TaskId, IoResponse)>,

    // Pending Searches
    pub pending_searches: ModuleGraphPendingSearchMap<'env>,

    pub diagnostics: &'env Diagnostics<'env>,
}

impl<'env> Executor<'env> {
    #[must_use]
    pub fn new(
        io_thread_pool: &'env ThreadPool,
        io_tx: mpsc::Sender<(TaskId, IoResponse)>,
        diagnostics: &'env Diagnostics<'env>,
    ) -> Self {
        Self {
            truth: RwLock::new(Truth::new()),
            injector: InjectorQueue::new(),
            num_scheduled: AtomicUsize::new(0),
            num_completed: AtomicUsize::new(0),
            num_queued: AtomicUsize::new(0),
            num_cleared: AtomicUsize::new(0),
            io_thread_pool,
            io_tx,
            pending_searches: Default::default(),
            diagnostics,
        }
    }

    #[must_use]
    pub fn spawn<T>(
        &self,
        execution: impl Into<Execution<'env>> + Executable<'env, Output = T>,
    ) -> Pending<'env, T>
    where
        T: UnwrapFrom<Artifact<'env>>,
    {
        Pending::new_unchecked(self.spawn_raw(execution))
    }

    #[must_use]
    pub fn spawn_raw(&self, execution: impl Into<Execution<'env>>) -> TaskRef<'env> {
        let execution = execution.into();
        let mut truth_guard = self.truth.write().unwrap();
        let truth = truth_guard.deref_mut();
        let tasks = &mut truth.tasks;
        self.push_unique_into_tasks(tasks, &[], execution)
    }

    #[must_use]
    pub fn request<R, T>(&self, request: R) -> Pending<'env, T>
    where
        R: Into<Request<'env>> + Executable<'env, Output = T>,
        T: UnwrapFrom<Artifact<'env>>,
    {
        Pending::new_unchecked(self.request_raw(request))
    }

    #[must_use]
    pub fn request_raw<R>(&self, request: R) -> TaskRef<'env>
    where
        R: Into<Request<'env>>,
    {
        let request = request.into();
        let mut truth_guard = self.truth.write().unwrap();
        let truth = truth_guard.deref_mut();

        let tasks = &mut truth.tasks;
        let requests = &mut truth.requests;

        *requests.entry(request).or_insert_with_key(|request| {
            let (prereqs, execution) = request.spawn();
            self.push_unique_into_tasks(tasks, &prereqs, execution)
        })
    }

    #[must_use]
    pub fn request_many<R, T>(
        &self,
        requests_iter: impl Iterator<Item = R>,
    ) -> BoxedSlice<Pending<'env, T>>
    where
        R: Into<Request<'env>> + Executable<'env, Output = T>,
        T: UnwrapFrom<Artifact<'env>>,
    {
        self.request_many_raw(requests_iter)
            .into_iter()
            .map(Pending::new_unchecked)
            .collect()
    }

    #[must_use]
    pub fn request_many_raw<R>(
        &self,
        requests_iter: impl Iterator<Item = R>,
    ) -> BoxedSlice<TaskRef<'env>>
    where
        R: Into<Request<'env>>,
    {
        let mut truth_guard = self.truth.write().unwrap();
        let truth = truth_guard.deref_mut();

        requests_iter
            .map(|request| {
                let tasks = &mut truth.tasks;
                let requests = &mut truth.requests;

                *requests
                    .entry(request.into())
                    .or_insert_with_key(|request| {
                        let (prereqs, execution) = request.spawn();
                        self.push_unique_into_tasks(tasks, &prereqs, execution)
                    })
            })
            .collect()
    }

    #[must_use]
    pub fn request_many_assoc<R, K, T>(
        &self,
        requests_iter: impl Iterator<Item = (K, R)>,
    ) -> BoxedSlice<(K, Pending<'env, T>)>
    where
        R: Into<Request<'env>> + Executable<'env, Output = T>,
        T: UnwrapFrom<Artifact<'env>>,
    {
        self.request_many_assoc_raw(requests_iter)
            .into_iter()
            .map(|(k, v)| (k, Pending::new_unchecked(v)))
            .collect()
    }

    #[must_use]
    pub fn request_many_assoc_raw<K, R>(
        &self,
        requests_iter: impl Iterator<Item = (K, R)>,
    ) -> BoxedSlice<(K, TaskRef<'env>)>
    where
        R: Into<Request<'env>>,
    {
        let mut truth_guard = self.truth.write().unwrap();
        let truth = truth_guard.deref_mut();

        requests_iter
            .map(|(key, request)| {
                let tasks = &mut truth.tasks;
                let requests = &mut truth.requests;

                (
                    key,
                    *requests
                        .entry(request.into())
                        .or_insert_with_key(|request| {
                            let (prereqs, execution) = request.spawn();
                            self.push_unique_into_tasks(tasks, &prereqs, execution)
                        }),
                )
            })
            .collect()
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
        for dependent in suspend_on {
            if tasks[*dependent].state.completed().is_none() {
                wait_on += 1;
            }
        }

        let new_task_ref = {
            let new_task_ref = tasks.alloc(Task {
                state: TaskState::Suspended(
                    execution.into(),
                    SuspendCondition::All(WaitingCount(wait_on)),
                ),
                dependents: vec![],
            });

            for dependent in suspend_on {
                if tasks[*dependent].state.completed().is_none() {
                    tasks[*dependent].dependents.push(new_task_ref);
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

    #[inline]
    pub fn demand<T>(&self, pending: Option<Pending<'env, T>>) -> Option<T>
    where
        T: UnwrapFrom<Artifact<'env>> + Clone,
    {
        pending.map(|pending| self.truth.read().unwrap().demand(pending).clone())
    }

    #[inline]
    pub fn demand_many<T>(&self, pending_list: &SuspendMany<'env, T>) -> Option<BoxedSlice<T>>
    where
        T: UnwrapFrom<Artifact<'env>> + Copy,
    {
        let truth = self.truth.read().unwrap();
        pending_list
            .as_ref()
            .map(|pending_list| truth.demand_many(pending_list.iter().copied()))
    }

    #[inline]
    pub fn demand_many_assoc<T, K>(
        &self,
        pending_list: &SuspendManyAssoc<'env, K, T>,
    ) -> Option<BoxedSlice<(K, T)>>
    where
        T: UnwrapFrom<Artifact<'env>> + Copy,
        K: Copy,
    {
        let truth = self.truth.read().unwrap();
        pending_list
            .as_ref()
            .map(|pending_list| truth.demand_many_assoc(pending_list.into_iter().copied()))
    }

    pub fn wake_pending_search(&self, graph_ref: ModuleGraphRef, name: &'env str) {
        let tasks_to_wake = self
            .pending_searches
            .get_or_default(graph_ref, |pending_search_map| {
                pending_search_map.wake(name)
            });

        self.num_queued
            .fetch_add(tasks_to_wake.len(), Ordering::SeqCst);

        for task_ref in tasks_to_wake {
            self.injector.push(task_ref);
        }
    }
}
