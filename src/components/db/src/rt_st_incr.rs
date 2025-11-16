use crate::{
    Artifact, BlockOn, Control, MustSuspend, NumberedRevision, Req, ReqInfo, ReqState, Rt, Thrd,
    WakeAfter, timeout::ShouldUnblock,
};
use smallvec::smallvec;
use std::collections::{HashMap, HashSet};
use std_ext::SmallVec4;

/// Single-Threaded Incremental Runtime
pub struct StIncrRt<'env> {
    queue: Vec<Req>,
    requests: HashMap<Req, ReqInfo<NumberedRevision>>,
    after_done_maybe_wake: HashMap<Req, Vec<Req>>,
    needs_before_wake: HashMap<Req, HashSet<Req>>,
    allocator: &'env bumpalo::Bump,
    current_revision: NumberedRevision,
}

impl<'env> StIncrRt<'env> {
    pub fn new(allocator: &'env bumpalo::Bump) -> Self {
        Self {
            queue: Default::default(),
            requests: Default::default(),
            after_done_maybe_wake: Default::default(),
            needs_before_wake: Default::default(),
            allocator,
            current_revision: NumberedRevision::default(),
        }
    }

    pub fn spawn(&mut self, child: Req, parent: Option<&Req>) {
        let is_complete = if let Some(running) = self.requests.get(&child) {
            running
                .state
                .as_ref()
                .expect("not processing")
                .is_complete()
        } else {
            self.requests
                .insert(child, ReqInfo::initial(self.current_revision));
            self.queue.push(child);
            false
        };

        if let Some(parent) = parent {
            if !is_complete {
                self.needs_before_wake
                    .entry(*parent)
                    .or_default()
                    .insert(child);
                self.after_done_maybe_wake
                    .entry(child)
                    .or_default()
                    .push(*parent);
            }
        }
    }
}

impl<'env> Rt for StIncrRt<'env> {
    type Revision = NumberedRevision;
    type StringKey = ();

    fn set_input(&mut self, req: Req, rev: Self::Revision, value: Artifact) {
        self.requests.insert(req, ReqInfo::complete(rev, value));
    }

    fn block_on(
        &mut self,
        initial_req: Req,
        mut timeout: impl ShouldUnblock,
    ) -> Result<BlockOn<&Artifact>, diagnostics::ErrorDiagnostic> {
        self.queue.push(initial_req);

        while let Some(req) = self.queue.pop() {
            let mut state = self
                .requests
                .entry(req)
                .or_insert_with(|| ReqInfo::initial(self.current_revision))
                .state
                .take()
                .expect("can process");

            let mut thrd = StIncrThrd {
                runtime: self,
                wake_after: vec![],
                spawn_detached: vec![],
            };

            let polled = req.poll(&mut thrd, &mut state);
            let wake_after = std::mem::take(&mut thrd.wake_after);
            let spawn_detached = std::mem::take(&mut thrd.spawn_detached);
            let is_complete = state.is_complete();
            drop(thrd);

            self.requests.get_mut(&req).unwrap().state = Some(state);

            if let Err(Control::Error(error)) = polled {
                return Err(error);
            }

            if is_complete {
                for child in self.after_done_maybe_wake.remove(&req).unwrap_or_default() {
                    let waiting_on = self.needs_before_wake.get_mut(&child).unwrap();
                    assert!(waiting_on.remove(&req));

                    if waiting_on.is_empty() {
                        self.queue.push(child);
                    }
                }
            } else {
                for new in wake_after {
                    self.spawn(new, Some(&req));
                }
            }

            for new in spawn_detached.into_iter() {
                self.spawn(new, None);
            }

            if timeout.should_unblock() {
                return Ok(BlockOn::TimedOut);
            }
        }

        Ok(self
            .requests
            .get(&initial_req)
            .and_then(|req_info| req_info.state.as_ref())
            .and_then(|state| state.as_complete())
            .map(BlockOn::Complete)
            .unwrap_or(BlockOn::Cyclic))
    }

    fn read_str<F, Ret>(&self, string_key: Self::StringKey, f: F) -> Ret
    where
        F: FnMut(&str) -> Ret,
    {
        todo!()
    }
}

pub struct StIncrThrd<'rt, 'env> {
    runtime: &'rt StIncrRt<'env>,
    wake_after: Vec<Req>,
    spawn_detached: Vec<Req>,
}

impl<'rt, 'env> Thrd for StIncrThrd<'rt, 'env> {
    type Runtime = StIncrRt<'env>;

    fn alloc_str(&mut self, content: &str) -> <Self::Runtime as Rt>::StringKey {
        todo!()
    }

    fn runtime(&self) -> &Self::Runtime {
        self.runtime
    }

    fn demand(&mut self, req: Req) -> Result<&Artifact, MustSuspend> {
        let Some(ReqInfo {
            state: Some(ReqState::Complete(artifact)),
            ..
        }) = self.runtime.requests.get(&req)
        else {
            self.wake_after.push(req);
            return Err(MustSuspend);
        };

        Ok(artifact)
    }

    fn anticipate(&mut self, req: Req) {
        self.spawn_detached.push(req);
    }
}
