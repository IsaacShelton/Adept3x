use crate::resolved::{self, Type};

#[derive(Clone, Debug)]
pub struct VariableStorage {
    pub types: Vec<Type>,
}

#[derive(Copy, Clone, Debug)]
pub struct VariableStorageKey {
    pub index: usize,
}

impl VariableStorage {
    pub fn new() -> Self {
        Self { types: vec![] }
    }

    pub fn add_variable(&mut self, resolved_type: resolved::Type) -> VariableStorageKey {
        let index = self.types.len();
        let key = VariableStorageKey { index };
        self.types.push(resolved_type);
        key
    }
}
