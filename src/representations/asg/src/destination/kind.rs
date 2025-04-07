use super::Destination;
use crate::{ArrayAccess, Expr, GlobalVariable, StructRef, Type, Variable};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum DestinationKind {
    Variable(Variable),
    GlobalVariable(GlobalVariable),
    Member {
        subject: Box<Destination>,
        struct_ref: StructRef,
        index: usize,
        field_type: Type,
    },
    ArrayAccess(Box<ArrayAccess>),
    Dereference(Expr),
}
