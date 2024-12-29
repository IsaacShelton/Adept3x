use crate::asg::{Destination, StructureRef, Type};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Member {
    pub subject: Destination,
    pub structure_ref: StructureRef,
    pub index: usize,
    pub field_type: Type,
}
