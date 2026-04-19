use crate::{
    Aft, CachedAft, IsDiv, IsImpure, Like, Minor, Req, RunDispatch, ShouldPersist, St, UnLike,
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
        + PartialEq
        + Eq
        + Like<Aft<Self>>
        + Cache<'e, Self>
        + From<Self::CachedAft<'e>>;
    type CachedAft<'e>: Serialize + for<'de> Deserialize<'de> + UnLike<CachedAft<Self>>;
    type St<'e>: Debug + Default + Send + Like<St>;
}

pub trait Cache<'e, P: Pf> {
    fn cache(&self) -> Option<&P::CachedAft<'e>>;
}
