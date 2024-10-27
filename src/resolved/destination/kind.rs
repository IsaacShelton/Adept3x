use crate::resolved::*;

#[derive(Clone, Debug)]
pub enum DestinationKind {
    Variable(Variable),
    GlobalVariable(GlobalVariable),
    Member {
        subject: Box<Destination>,
        structure_ref: StructureRef,
        index: usize,
        field_type: Type,
    },
    ArrayAccess(Box<ArrayAccess>),
    Dereference(Expr),
}
