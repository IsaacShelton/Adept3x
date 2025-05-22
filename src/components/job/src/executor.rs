/*
    ====================  components/job/src/executor.rs  =====================
    Defines what a worker sees as it's `Executor`.

    This only contains parts of the executor that workers are allowed to see.

    Data that must be kept inaccessible to workers is instead kept in `MainExecutor`.
    ---------------------------------------------------------------------------
*/

use crate::{
    Artifact, Executable, Execution, Pending, Request, Spawnable, SuspendCondition, SuspendMany,
    SuspendManyAssoc, Task, TaskId, TaskRef, TaskState, Truth, UnwrapFrom, WaitingCount,
};
use arena::Arena;
use crossbeam_deque::Injector as InjectorQueue;
use std::{
    ops::DerefMut,
    sync::{
        RwLock,
        atomic::{AtomicUsize, Ordering},
    },
};
use std_ext::BoxedSlice;

pub struct Executor<'env> {
    pub injector: InjectorQueue<TaskRef<'env>>,
    pub truth: RwLock<Truth<'env>>,
    pub num_completed: AtomicUsize,
    pub num_scheduled: AtomicUsize,
    pub num_queued: AtomicUsize,
    pub num_cleared: AtomicUsize,
}

impl<'env> Executor<'env> {
    #[must_use]
    pub fn new() -> Self {
        Self {
            truth: RwLock::new(Truth::new()),
            injector: InjectorQueue::new(),
            num_scheduled: AtomicUsize::new(0),
            num_completed: AtomicUsize::new(0),
            num_queued: AtomicUsize::new(0),
            num_cleared: AtomicUsize::new(0),
        }
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
        pending_list.as_ref().map(|pending_list| {
            self.truth
                .read()
                .unwrap()
                .demand_many(pending_list.iter().copied())
        })
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
        pending_list.as_ref().map(|pending_list| {
            self.truth
                .read()
                .unwrap()
                .demand_many_assoc(pending_list.into_iter().copied())
        })
    }
}
