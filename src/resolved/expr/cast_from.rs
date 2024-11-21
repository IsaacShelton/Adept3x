use super::Cast;
use crate::resolved::Type;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct CastFrom {
    pub cast: Cast,
    pub from_type: Type,
}
