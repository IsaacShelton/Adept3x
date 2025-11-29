use crate::{Aft, Pf, Req, St};

pub trait Like<T> {
    fn like(self) -> T;
    fn like_ref(&self) -> &T;
    fn like_mut(&mut self) -> &mut T;
}

impl<'e> Like<Req<'e>> for Req<'e> {
    #[inline(always)]
    fn like(self) -> Self {
        self
    }

    #[inline(always)]
    fn like_ref(&self) -> &Self {
        self
    }

    #[inline(always)]
    fn like_mut(&mut self) -> &mut Self {
        self
    }
}

impl<'e> Like<St> for St {
    #[inline(always)]
    fn like(self) -> Self {
        self
    }

    #[inline(always)]
    fn like_ref(&self) -> &Self {
        self
    }

    #[inline(always)]
    fn like_mut(&mut self) -> &mut Self {
        self
    }
}

impl<'e, P: Pf> Like<Aft<P>> for Aft<P> {
    #[inline(always)]
    fn like(self) -> Self {
        self
    }

    #[inline(always)]
    fn like_ref(&self) -> &Self {
        self
    }

    #[inline(always)]
    fn like_mut(&mut self) -> &mut Self {
        self
    }
}
