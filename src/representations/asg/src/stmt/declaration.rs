use crate::{Expr, VariableStorageKey};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Declaration {
    pub key: VariableStorageKey,
    pub value: Option<Expr>,
}
