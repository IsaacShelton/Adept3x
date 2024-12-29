use crate::asg::{Type, VariableStorageKey};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Variable {
    pub key: VariableStorageKey,
    pub resolved_type: Type,
}
