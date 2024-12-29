use crate::asg::*;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum DestinationKind {
    Variable(Variable),
    GlobalVariable(GlobalVariable),
    Member {
        subject: Box<Destination>,
        structure_ref: StructRef,
        index: usize,
        field_type: Type,
    },
    ArrayAccess(Box<ArrayAccess>),
    Dereference(Expr),
}
