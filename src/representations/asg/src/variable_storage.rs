use super::Type;

#[derive(Clone, Debug)]
pub struct VariableStorage {
    pub instances: Vec<VariableInstance>,
    pub num_params: usize,
}

#[derive(Clone, Debug)]
pub struct VariableInstance {
    pub ty: Type,
}

impl VariableInstance {
    pub fn new(ty: Type) -> Self {
        Self { ty }
    }
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq)]
pub struct VariableStorageKey {
    pub index: usize,
}

impl VariableStorage {
    pub fn new() -> Self {
        Self {
            instances: vec![],
            num_params: 0,
        }
    }

    pub fn add_variable(&mut self, ty: Type) -> VariableStorageKey {
        let index = self.instances.len();
        let key = VariableStorageKey { index };
        self.instances.push(VariableInstance::new(ty));
        key
    }

    pub fn add_param(&mut self, ty: Type) -> VariableStorageKey {
        assert_eq!(self.num_params, self.instances.len());
        self.num_params += 1;
        self.add_variable(ty)
    }

    pub fn get(&self, key: VariableStorageKey) -> Option<&VariableInstance> {
        self.instances.get(key.index)
    }

    pub fn count(&self) -> usize {
        self.instances.len()
    }
}
