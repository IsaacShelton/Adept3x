use crate::{
    Aft, Approach, AsSyms, IsDiv, IsImpure, Like, Minor, Req, Run, RunDispatch, St, Suspend, Th,
    UnLike,
};
use std::{fmt::Debug, hash::Hash};

pub trait Pf: Clone + Debug + Default {
    type Req<'e>: Clone
        + Debug
        + Hash
        + PartialEq
        + Eq
        + Send
        + IsImpure
        + From<Approach>
        + RunDispatch<'e, Self>
        + UnLike<Req<'e>>;
    type Rev: Copy
        + Clone
        + Debug
        + Default
        + PartialEq
        + Eq
        + PartialOrd
        + Ord
        + Send
        + IsDiv
        + Minor;
    type Aft<'e>: Debug + Clone + Send + AsSyms<Self> + PartialEq + Eq + Like<Aft<Self>>;
    type St<'e>: Debug + Default + Send + Like<St>;
}

impl<'e, P: Pf> RunDispatch<'e, P> for P::Req<'e>
where
    P::Req<'e>: Like<Req<'e>>,
    P::Aft<'e>: UnLike<Aft<P>>,
{
    fn run_dispath(
        &self,
        st: &mut P::St<'e>,
        th: &mut impl Th<'e, P>,
    ) -> Result<P::Aft<'e>, Suspend<'e, P>> {
        match self.like_ref() {
            Req::FindProjectConfig(req) => {
                req.run(st, th).map(|aft| P::Aft::un_like(Aft::from(aft)))
            }
            Req::GetProject(req) => req.run(st, th).map(|aft| P::Aft::un_like(Aft::from(aft))),
            Req::GetRootSourceFile(req) => {
                req.run(st, th).map(|aft| P::Aft::un_like(Aft::from(aft)))
            }
            Req::Approach(req) => req.run(st, th).map(|aft| P::Aft::un_like(Aft::from(aft))),
            Req::Search(req) => req.run(st, th).map(|aft| P::Aft::un_like(Aft::from(aft))),
            Req::ListSymbols(req) => req.run(st, th).map(|aft| P::Aft::un_like(Aft::from(aft))),
        }
    }
}
