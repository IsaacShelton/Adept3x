use crate::{Aft, Approach, Like, Pf, Syms, UnwrapAft};

pub trait AsSyms<P: Pf> {
    fn as_syms(&self) -> Option<&Syms<P>>;
}

impl<'e, P: Pf> AsSyms<P> for P::Aft<'e>
where
    P::Aft<'e>: Like<Aft<P>>,
{
    fn as_syms(&self) -> Option<&Syms<P>> {
        Approach::as_aft(self.like_ref())
    }
}
