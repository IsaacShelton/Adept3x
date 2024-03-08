use crate::resolved::{self, Type};

#[derive(Clone, Debug)]
pub struct VariableStorage {
    pub types: Vec<Type>,
    pub num_parameters: usize,
}

#[derive(Copy, Clone, Debug)]
pub struct VariableStorageKey {
    pub index: usize,
}

impl VariableStorage {
    pub fn new() -> Self {
        Self {
            types: vec![],
            num_parameters: 0,
        }
    }

    pub fn add_variable(&mut self, resolved_type: resolved::Type) -> VariableStorageKey {
        let index = self.types.len();
        let key = VariableStorageKey { index };
        self.types.push(resolved_type);
        key
    }

    pub fn add_parameter(&mut self, resolved_type: resolved::Type) -> VariableStorageKey {
        assert_eq!(self.num_parameters, self.types.len());

        self.num_parameters += 1;
        self.add_variable(resolved_type)
    }
}
