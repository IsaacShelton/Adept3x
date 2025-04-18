use super::Type;
use std::cell::OnceCell;

#[derive(Clone, Debug)]
pub struct VariableStorage {
    pub instances: Vec<VariableInstance>,
    pub num_params: usize,
}

#[derive(Clone, Debug)]
pub struct VariableInstance {
    pub ty: Type,
    pub initialized: OnceCell<()>,
}

impl VariableInstance {
    pub fn new(ty: Type, is_initialized: bool) -> Self {
        let initialized = if is_initialized {
            OnceCell::from(())
        } else {
            OnceCell::new()
        };

        Self { ty, initialized }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.get().is_some()
    }

    pub fn set_initialized(&self) {
        let _ = self.initialized.set(());
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

    pub fn add_variable(&mut self, ty: Type, is_initialized: bool) -> VariableStorageKey {
        let index = self.instances.len();
        let key = VariableStorageKey { index };
        self.instances
            .push(VariableInstance::new(ty, is_initialized));
        key
    }

    pub fn add_param(&mut self, ty: Type) -> VariableStorageKey {
        assert_eq!(self.num_params, self.instances.len());
        self.num_params += 1;
        self.add_variable(ty, true)
    }

    pub fn get(&self, key: VariableStorageKey) -> Option<&VariableInstance> {
        self.instances.get(key.index)
    }

    pub fn count(&self) -> usize {
        self.instances.len()
    }
}
