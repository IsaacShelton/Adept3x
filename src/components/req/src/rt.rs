use crate::{
    BlockOn, Pf, Req, ShouldUnblock, Suspend, Syms, Task, TopErrorsNode, UnLike, UnwrapAft,
};
use vfs::Vfs;

pub enum QueryMode {
    New,
    Continue,
}

pub trait Rt<'e, P: Pf>: Send {
    type Query: Send;
    fn query(&mut self, req: P::Req<'e>, mode: QueryMode) -> Self::Query;
    fn block_on(
        &mut self,
        query: Self::Query,
        timeout: impl ShouldUnblock,
    ) -> Result<BlockOn<P::Aft<'e>, Self::Query>, TopErrorsNode>;
    fn syms(&self) -> Syms<P>;
    fn vfs(&self) -> &Vfs;
}

pub trait Th<'e, P: Pf>
where
    P::Req<'e>: UnLike<Req<'e>>,
{
    type Rt: Rt<'e, P>;
    fn rt(&self) -> &Self::Rt;
    fn demand<R>(&mut self, req: R) -> Result<&R::Aft<'e>, Suspend>
    where
        R: Into<Req<'e>> + UnwrapAft<'e, P>;
}

pub trait Ch<'e, P: Pf> {
    fn acq(&self, req: &P::Req<'e>) -> (Task<'e, P>, P::St<'e>);
    fn rel(&self, req: &P::Req<'e>, st: P::St<'e>);
    fn get(&self, req: &P::Req<'e>) -> Option<&P::Aft<'e>>;
}
