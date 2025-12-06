use crate::{Aft, Pf, Req, St};

pub trait Like<T> {
    fn like(self) -> T;
    fn like_ref(&self) -> &T;
    fn like_mut(&mut self) -> &mut T;
}

impl Like<Req> for Req {
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

impl Like<St> for St {
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

impl<P: Pf> Like<Aft<P>> for Aft<P> {
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
