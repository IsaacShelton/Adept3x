use crate::resolved::{Expr, VariableStorageKey};

#[derive(Clone, Debug)]
pub struct Declaration {
    pub key: VariableStorageKey,
    pub value: Option<Expr>,
}
