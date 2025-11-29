use crate::{Aft, Approach, AsSyms, IsDiv, IsImpure, Like, Minor, Req, RunDispatch, St, UnLike};
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
