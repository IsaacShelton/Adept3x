use crate::resolved::*;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
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
