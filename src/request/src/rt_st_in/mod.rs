mod query;
mod react;
mod req_cache;
mod wake_dependants;

use crate::{
    BlockOn, Major, Pf, QueryMode, Rt, ShouldUnblock, TaskStatus, TaskStatusKind, TopErrorsNode,
    rt_st_in::query::{QueryFiles, RtStInQuery},
    rt_trace,
};
use react::*;
pub use req_cache::*;
use std::collections::HashMap;
pub use wake_dependants::*;

pub struct RtStIn<'e, P: Pf>
where
    P::Rev: Major,
{
    pub(crate) cache: ReqCache<'e, P>,
    pub(crate) current: P::Rev,
    pub cache_to_disk: bool,
}

impl<'e, P: Pf> RtStIn<'e, P>
where
    P::Rev: Major,
{
    pub fn new(cache: ReqCache<'e, P>) -> Self {
        Self {
            cache,
            current: P::Rev::default(),
            cache_to_disk: false,
        }
    }

    pub fn cache(&self) -> &ReqCache<'e, P> {
        &self.cache
    }
}

impl<'e, P: Pf> Rt<'e, P> for RtStIn<'e, P>
where
    P::Rev: Major,
{
    type Query = RtStInQuery<'e, P>;

    fn query(&mut self, req: P::Req<'e>, mode: QueryMode) -> RtStInQuery<'e, P> {
        if let QueryMode::New = mode {
            self.current = self.current.major();
            rt_trace!("Currently at: {:?}", self.current);
        }

        RtStInQuery {
            queue: vec![req.clone()],
            waiting: HashMap::new(),
            rev: self.current,
            req,
            files: QueryFiles::default(),
        }
    }

    fn current(&self) -> P::Rev {
        self.current
    }

    fn block_on(
        &mut self,
        mut query: Self::Query,
        mut timeout: impl ShouldUnblock,
    ) -> Result<BlockOn<P::Aft<'e>, Self::Query>, TopErrorsNode> {
        while let Some(req) = query.queue.pop() {
            react(self, &mut query, req);

            if timeout.should_unblock() {
                return Ok(BlockOn::TimedOut(query));
            }
        }

        let status = self.cache.get(&query.req);

        match status {
            Some(Some(TaskStatus {
                kind: TaskStatusKind::Completed(completed),
                ..
            })) => Ok(BlockOn::Complete(completed.aft.clone())),
            _ => {
                unreachable!("block_on should have completed task since nothing left in queue");
            }
        }
    }
}
