mod cache;
mod query;
mod react;
mod wake_dependants;

use crate::{
    Approach, BlockOn, Errs, IsDiv, Major, Minor, Pf, Rt, ShouldUnblock, Syms, TaskStatusKind,
    rt_st_in::query::RtStInQuery,
};
pub use cache::*;
use react::*;
use std::collections::HashMap;
use vfs::Vfs;
pub use wake_dependants::*;

pub struct RtStIn<'e, P: Pf>
where
    P::Rev: Major,
{
    pub(crate) cache: Cache<'e, P>,
    pub(crate) syms: Syms<P>,
    pub(crate) current: P::Rev,
    pub(crate) vfs: Vfs,
}

impl<'e, P: Pf> RtStIn<'e, P>
where
    P::Rev: Major,
{
    pub fn new(cache: Cache<'e, P>) -> Self {
        Self {
            cache,
            syms: Syms::default(),
            current: P::Rev::default(),
            vfs: Vfs::default(),
        }
    }
}

impl<'e, P: Pf> Rt<'e, P> for RtStIn<'e, P>
where
    P::Rev: Major,
{
    type Query = RtStInQuery<'e, P>;

    fn query(&mut self, req: P::Req<'e>) -> RtStInQuery<'e, P> {
        self.current = self.current.major();
        RtStInQuery {
            queue: vec![Approach.into(), req.clone()],
            waiting: HashMap::new(),
            rev: self.current,
            req,
            new_syms: None,
        }
    }

    fn syms(&self) -> Syms<P> {
        self.syms.clone()
    }

    fn vfs(&self) -> &Vfs {
        &self.vfs
    }

    fn block_on(
        &mut self,
        mut query: Self::Query,
        mut timeout: impl ShouldUnblock,
    ) -> Result<BlockOn<P::Aft<'e>, Self::Query>, Errs> {
        loop {
            while let Some(req) = query.queue.pop() {
                react(self, &mut query, req);
            }

            let max_iters = 1000;
            let Some(new_syms) = query.new_syms.take() else {
                let aft = match &self
                    .cache
                    .get(&query.req)
                    .expect("request existed")
                    .as_ref()
                    .expect("not running")
                    .kind
                {
                    TaskStatusKind::Running(..) | TaskStatusKind::Restarting(..) => {
                        panic!("expected task to be completed")
                    }
                    TaskStatusKind::Completed(completed) => completed.aft.clone(),
                };

                return Ok(BlockOn::Complete(aft));
            };

            if self.current.is_div(max_iters) {
                return Ok(BlockOn::Diverges);
            }

            if timeout.should_unblock() {
                return Ok(BlockOn::TimedOut(query));
            }

            self.syms = new_syms;
            self.current = self.current.minor();
            query.queue.push(query.req.clone());
            query.queue.push(Approach.into());
        }
    }
}
