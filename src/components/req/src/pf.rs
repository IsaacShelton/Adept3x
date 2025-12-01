use crate::{
    Aft, Approach, AsSyms, IsDiv, IsImpure, Like, Minor, Req, RunDispatch, ShouldPersist, St,
    UnLike,
};
use serde::{Deserialize, Serialize};
use std::{fmt::Debug, hash::Hash};

pub trait Pf: Clone + Debug + Default {
    type Req<'e>: Clone
        + Debug
        + Hash
        + PartialEq
        + Eq
        + Send
        + IsImpure
        + ShouldPersist
        + From<Approach>
        + RunDispatch<'e, Self>
        + UnLike<Req>
        + Serialize
        + Deserialize<'e>;
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
        + Minor
        + Serialize
        + for<'de> Deserialize<'de>;
    type Aft<'e>: Debug
        + Clone
        + Send
        + AsSyms<Self>
        + PartialEq
        + Eq
        + Like<Aft<Self>>
        + Serialize
        + Deserialize<'e>;
    type St<'e>: Debug + Default + Send + Like<St>;
}
