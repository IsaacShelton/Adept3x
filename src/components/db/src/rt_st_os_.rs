use crate::{Artifact, Control, MustSuspend, Req, ReqInfo, ReqState, Rt, Thrd, WakeAfter};
use smallvec::smallvec;
use std::collections::{HashMap, HashSet};
use std_ext::SmallVec4;

pub struct SingleThreadedOneshotRt<'env> {
    queue: Vec<Req>,
    requests: HashMap<Req, ReqInfo<()>>,
    after_done_maybe_wake: HashMap<Req, Vec<Req>>,
    needs_before_wake: HashMap<Req, HashSet<Req>>,
    allocator: &'env bumpalo::Bump,
}

impl<'env> SingleThreadedOneshotRt<'env> {
    pub fn new(allocator: &'env bumpalo::Bump) -> Self {
        Self {
            queue: Default::default(),
            requests: Default::default(),
            after_done_maybe_wake: Default::default(),
            needs_before_wake: Default::default(),
            allocator,
        }
    }
}

impl<'env> Rt for SingleThreadedOneshotRt<'env> {
    type Revision = ();
    type StringKey = ();

    fn set_input(&mut self, req: Req, rev: Self::Revision, value: Artifact) {
        self.requests.insert(req, ReqInfo::complete(rev, value));
    }

    fn block_on(
        &mut self,
        initial_req: Req,
    ) -> Result<Option<&Artifact>, diagnostics::ErrorDiagnostic> {
        self.queue.push(initial_req);

        while let Some(req) = self.queue.pop() {
            let req_info = self
                .requests
                .entry(req)
                .or_insert_with(|| ReqInfo::initial(()));

            let mut state = req_info.state.take().expect("can process");

            let mut thrd = SingleThreadedOneshotThrd {
                runtime: self,
                wake_after: vec![],
            };

            let polled = req.poll(&mut thrd, &mut state);
            let is_complete = state.is_complete();
            let wake_after = std::mem::take(&mut thrd.wake_after);
            self.requests.get_mut(&req).unwrap().state = Some(state);

            match polled {
                Ok(()) | Err(Control::Suspend) if is_complete => {
                    for dependant in self.after_done_maybe_wake.remove(&req).unwrap_or_default() {
                        let waiting_on = self.needs_before_wake.get_mut(&dependant).unwrap();
                        assert!(waiting_on.remove(&req));

                        if waiting_on.is_empty() {
                            self.queue.push(dependant);
                        }
                    }
                }
                Ok(()) | Err(Control::Suspend) => {
                    for requirement in wake_after {
                        let before_wake = self.needs_before_wake.entry(req).or_default();
                        before_wake.insert(requirement);

                        self.after_done_maybe_wake
                            .entry(requirement)
                            .or_default()
                            .push(req);

                        if self.requests.get(&requirement).is_none() {
                            self.requests.insert(requirement, ReqInfo::initial(()));
                            self.queue.push(requirement);
                        }
                    }
                }
                Err(Control::Error(error)) => return Err(error),
            }
        }

        Ok(self
            .requests
            .get(&initial_req)
            .and_then(|req_info| req_info.state.as_ref())
            .and_then(|state| state.as_complete()))
    }

    fn read_str<F, Ret>(&self, string_key: Self::StringKey, f: F) -> Ret
    where
        F: FnMut(&str) -> Ret,
    {
        todo!()
    }
}

pub struct SingleThreadedOneshotThrd<'rt, 'env> {
    runtime: &'rt SingleThreadedOneshotRt<'env>,
    wake_after: Vec<Req>,
}

impl<'rt, 'env> Thrd for SingleThreadedOneshotThrd<'rt, 'env> {
    type Runtime = SingleThreadedOneshotRt<'env>;

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
}
