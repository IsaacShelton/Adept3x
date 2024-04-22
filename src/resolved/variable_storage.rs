use crate::resolved::{self, Type};
use std::cell::OnceCell;

#[derive(Clone, Debug)]
pub struct VariableStorage {
    pub instances: Vec<VariableInstance>,
    pub num_parameters: usize,
}

#[derive(Clone, Debug)]
pub struct VariableInstance {
    pub resolved_type: Type,
    initialized: OnceCell<()>,
}

impl VariableInstance {
    pub fn new(resolved_type: Type, is_initialized: bool) -> Self {
        let initialized = if is_initialized {
            OnceCell::from(())
        } else {
            OnceCell::new()
        };

        Self {
            resolved_type,
            initialized,
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized.get().is_some()
    }

    pub fn set_initialized(&self) {
        let _ = self.initialized.set(());
    }
}

#[derive(Copy, Clone, Debug)]
pub struct VariableStorageKey {
    pub index: usize,
}

impl VariableStorage {
    pub fn new() -> Self {
        Self {
            instances: vec![],
            num_parameters: 0,
        }
    }

    pub fn add_variable(
        &mut self,
        resolved_type: resolved::Type,
        is_initialized: bool,
    ) -> VariableStorageKey {
        let index = self.instances.len();
        let key = VariableStorageKey { index };
        self.instances
            .push(VariableInstance::new(resolved_type, is_initialized));
        key
    }

    pub fn add_parameter(&mut self, resolved_type: resolved::Type) -> VariableStorageKey {
        assert_eq!(self.num_parameters, self.instances.len());
        self.num_parameters += 1;
        self.add_variable(resolved_type, true)
    }

    pub fn get(&self, key: VariableStorageKey) -> Option<&VariableInstance> {
        self.instances.get(key.index)
    }

    pub fn count(&self) -> usize {
        self.instances.len()
    }
}
