use super::Expr;
use crate::resolved::{Type, VariableStorageKey};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct DeclareAssign {
    pub key: VariableStorageKey,
    pub value: Expr,
    pub resolved_type: Type,
}
