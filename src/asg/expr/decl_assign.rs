use super::Expr;
use crate::asg::{Type, VariableStorageKey};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct DeclareAssign {
    pub key: VariableStorageKey,
    pub value: Expr,
    pub ty: Type,
}
