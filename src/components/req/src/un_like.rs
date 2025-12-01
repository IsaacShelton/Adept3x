use crate::{Aft, Pf, Req};

pub trait UnLike<T> {
    fn un_like(value: T) -> Self;
    fn un_like_ref(value: &T) -> &Self;
    fn un_like_mut(value: &mut T) -> &mut Self;
}

impl<P: Pf> UnLike<Aft<P>> for Aft<P> {
    #[inline(always)]
    fn un_like(value: Aft<P>) -> Self {
        value
    }

    #[inline(always)]
    fn un_like_ref(value: &Aft<P>) -> &Self {
        value
    }

    #[inline(always)]
    fn un_like_mut(value: &mut Aft<P>) -> &mut Self {
        value
    }
}

impl UnLike<Req> for Req {
    #[inline(always)]
    fn un_like(value: Req) -> Self {
        value
    }

    #[inline(always)]
    fn un_like_ref(value: &Self) -> &Self {
        value
    }

    #[inline(always)]
    fn un_like_mut(value: &mut Self) -> &mut Self {
        value
    }
}
