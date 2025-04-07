use crate::{Cast, Type};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CastFrom {
    pub cast: Cast,
    pub from_type: Type,
}
