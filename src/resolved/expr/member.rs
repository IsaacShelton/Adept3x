use crate::resolved::{Destination, StructureRef, Type};

#[derive(Clone, Debug)]
pub struct Member {
    pub subject: Destination,
    pub structure_ref: StructureRef,
    pub index: usize,
    pub field_type: Type,
}
