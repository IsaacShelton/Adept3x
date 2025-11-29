use crate::{
    AsSyms, Completed, IsImpure, Like, Major, Pf, Req, Restarting, RtStIn, RunDispatch, Running,
    Suspend, Task, TaskStatus, TaskStatusKind, Th, UnLike, UnwrapAft, rt_st_in::query::RtStInQuery,
    wake_dependants,
};

pub fn react<'e, P: Pf>(rt: &mut RtStIn<'e, P>, query: &mut RtStInQuery<'e, P>, req: P::Req<'e>)
where
    P::Rev: Major,
{
    // If the task has never been run before, start it
    let entry = rt.cache.entry(&req).or_insert_with(|| {
        Some(TaskStatus {
            kind: TaskStatusKind::Running(Running {
                st: P::St::default(),
                prev_aft: None,
                left_waiting_on: 0,
            }),
            task: Task {
                verified_at: rt.current,
                changed_at: rt.current,
                requested: vec![],
            },
        })
    });

    // Acquire running task
    let mut status = entry.take().expect("task to not be processing");

    let (running, task) = match status.kind {
        TaskStatusKind::Running(running) => (running, status.task),
        TaskStatusKind::Completed(completed) => {
            if status.task.verified_at < rt.current {
                println!("Not verified for this rev yet {:?}", req);
                rt.cache.insert(
                    req.clone(),
                    Some(TaskStatus {
                        kind: TaskStatusKind::Restarting(Restarting {
                            prev_aft: completed.aft,
                            left_waiting_on: 0,
                            verified_at: status.task.verified_at,
                            deps_ready: false,
                        }),
                        task: Task {
                            verified_at: rt.current,
                            changed_at: status.task.changed_at,
                            requested: status.task.requested,
                        },
                    }),
                );

                query.queue.push(req.clone());
            } else {
                println!("Already verified");
                status.task.verified_at = rt.current;
                *entry = Some(TaskStatus {
                    kind: TaskStatusKind::Completed(completed),
                    task: status.task,
                });
            }
            return;
        }
        TaskStatusKind::Restarting(restarting) => {
            if !restarting.deps_ready {
                let mut left_waiting_on = 0;

                for dep in status.task.requested.iter() {
                    let dep_status = rt
                        .cache
                        .get(dep)
                        .expect("dependency to have run sometime previously");

                    match dep_status {
                        Some(TaskStatus {
                            kind: TaskStatusKind::Completed(completed),
                            task: dep_task,
                        }) => {
                            if dep_task.verified_at < rt.current {
                                rt.cache.insert(
                                    req.clone(),
                                    Some(TaskStatus {
                                        kind: TaskStatusKind::Restarting(Restarting {
                                            prev_aft: completed.aft.clone(),
                                            left_waiting_on: 0,
                                            verified_at: dep_task.verified_at,
                                            deps_ready: false,
                                        }),
                                        task: Task {
                                            verified_at: rt.current,
                                            changed_at: dep_task.changed_at,
                                            requested: dep_task.requested.clone(),
                                        },
                                    }),
                                );
                                query.queue.push(dep.clone());
                                query
                                    .waiting
                                    .entry(dep.clone())
                                    .or_default()
                                    .push(req.clone());
                                left_waiting_on += 1;
                            }
                        }
                        Some(TaskStatus {
                            kind: TaskStatusKind::Running(..) | TaskStatusKind::Restarting(..),
                            ..
                        })
                        | None => {
                            query
                                .waiting
                                .entry(dep.clone())
                                .or_default()
                                .push(req.clone());
                            left_waiting_on += 1;
                        }
                    }
                }

                if left_waiting_on != 0 {
                    rt.cache.insert(
                        req.clone(),
                        Some(TaskStatus {
                            kind: TaskStatusKind::Restarting(Restarting {
                                prev_aft: restarting.prev_aft,
                                left_waiting_on: left_waiting_on,
                                verified_at: restarting.verified_at,
                                deps_ready: true,
                            }),
                            task: status.task,
                        }),
                    );
                    return;
                }
            }

            let needs_to_be_recomputed = req.is_impure()
                || status.task.requested.iter().any(|req| {
                    rt.cache
                        .get(req)
                        .expect("dependency has been previously requested")
                        .as_ref()
                        .expect("dependency is not active")
                        .task
                        .changed_at
                        > restarting.verified_at
                });

            if !needs_to_be_recomputed {
                wake_dependants(rt, query, &req);

                rt.cache.insert(
                    req,
                    Some(TaskStatus {
                        kind: TaskStatusKind::Completed(Completed {
                            aft: restarting.prev_aft,
                        }),
                        task: Task {
                            verified_at: rt.current,
                            changed_at: status.task.changed_at,
                            requested: status.task.requested,
                        },
                    }),
                );
                return;
            }

            (
                Running {
                    st: P::St::default(),
                    prev_aft: Some(restarting.prev_aft),
                    left_waiting_on: restarting.left_waiting_on,
                },
                Task {
                    verified_at: rt.current,
                    changed_at: status.task.changed_at,
                    requested: status.task.requested,
                },
            )
        }
    };

    // Process the task
    let new_task_status = run_in_th(rt, query, &req, task, running);
    rt.cache.insert(req, Some(new_task_status));
}

pub struct ThStIn<'rt, 'e, P: Pf>
where
    P::Rev: Major,
{
    rt: &'rt RtStIn<'e, P>,
}

impl<'rt, 'e, P: Pf> ThStIn<'rt, 'e, P>
where
    P::Rev: Major,
{
    pub fn new(rt: &'rt RtStIn<'e, P>) -> Self {
        Self { rt }
    }
}

impl<'rt, 'e, P: Pf> Th<'e, P> for ThStIn<'rt, 'e, P>
where
    P::Rev: Major,
{
    type Rt = RtStIn<'e, P>;

    fn rt(&self) -> &Self::Rt {
        self.rt
    }

    fn demand<R>(&mut self, req: R) -> Result<&R::Aft<'e>, Suspend<'e, P>>
    where
        R: Into<Req<'e>> + UnwrapAft<'e, P>,
    {
        let req = req.into();
        eprintln!("Requesting {:?}", req);

        let existing = self.rt.cache.get(P::Req::un_like_ref(&req));

        let Some(Some(TaskStatus {
            kind: TaskStatusKind::Completed(completed),
            task,
        })) = existing
        else {
            eprintln!("  It's not ready");
            return Err(vec![P::Req::un_like(req)]);
        };

        if task.verified_at < self.rt.current {
            eprintln!("  It's out of date");
            return Err(vec![P::Req::un_like(req)]);
        }

        eprintln!("  It's verified for this revision");
        Ok(R::as_aft(&completed.aft.like_ref()).unwrap())
    }
}

fn run_in_th<'e, P: Pf>(
    rt: &mut RtStIn<'e, P>,
    query: &mut RtStInQuery<'e, P>,
    req: &P::Req<'e>,
    task: Task<'e, P>,
    mut running: Running<'e, P>,
) -> TaskStatus<'e, P>
where
    P::Rev: Major,
{
    let st = &mut running.st;

    eprintln!("Processing {:?}, {:?}", req, &query.queue);
    let mut th = ThStIn::new(rt);
    let result = req.run_dispath(st, &mut th);

    // Check the result
    match result {
        Ok(aft) => {
            if let Some(new_syms) = aft.as_syms() {
                if rt.syms.has_changed(new_syms) {
                    query.new_syms = Some(new_syms.clone());
                }
            }

            let mut task = task;
            if Some(&aft) != running.prev_aft.as_ref() {
                task.changed_at = rt.current;
            }

            wake_dependants(rt, query, req);

            TaskStatus {
                kind: TaskStatusKind::Completed(Completed { aft }),
                task,
            }
        }
        Err(deps) => {
            eprintln!("  It has outdated dependencies");
            running.left_waiting_on = 0;

            for dep in &deps {
                match rt.cache.get(&dep) {
                    Some(Some(TaskStatus {
                        kind: TaskStatusKind::Completed(..),
                        task: dep_task,
                    })) => {
                        if dep_task.verified_at >= query.rev {
                            eprintln!("  Dependency is already verified");
                            continue;
                        } else {
                            eprintln!("  Dependency is stale");
                            // The completed result is stale, we need to requeue it
                            query.queue.push(dep.clone());
                            query
                                .waiting
                                .entry(dep.clone())
                                .or_default()
                                .push(req.clone());
                            running.left_waiting_on += 1;
                        }
                    }
                    Some(
                        None
                        | Some(TaskStatus {
                            kind: TaskStatusKind::Running(..) | TaskStatusKind::Restarting(..),
                            ..
                        }),
                    ) => {
                        eprintln!("  Dependency is already being processed");
                        // Already in queue/being processed
                        query
                            .waiting
                            .entry(dep.clone())
                            .or_default()
                            .push(req.clone());
                        running.left_waiting_on += 1;
                    }
                    None => {
                        eprintln!("  Dependency has not started yet {:?}", &query.queue);
                        // Has never been invoked
                        query.queue.push(dep.clone());
                        query
                            .waiting
                            .entry(dep.clone())
                            .or_default()
                            .push(req.clone());
                        running.left_waiting_on += 1;
                    }
                }
            }

            let mut task = task;
            task.requested.extend(deps);

            // Re-queue immediately if everything requested is already ready and valid
            if running.left_waiting_on == 0 {
                eprintln!("  No dependencies need to be waited on");
                query.queue.push(req.clone());
            }

            TaskStatus {
                kind: TaskStatusKind::Running(running),
                task,
            }
        }
    }
}
