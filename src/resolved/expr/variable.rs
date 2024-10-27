use crate::resolved::{Type, VariableStorageKey};

#[derive(Clone, Debug)]
pub struct Variable {
    pub key: VariableStorageKey,
    pub resolved_type: Type,
}
