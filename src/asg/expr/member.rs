use crate::asg::{Destination, StructRef, Type};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Member {
    pub subject: Destination,
    pub structure_ref: StructRef,
    pub index: usize,
    pub field_type: Type,
}
