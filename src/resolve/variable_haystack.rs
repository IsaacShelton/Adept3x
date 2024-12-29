use crate::asg::{self, VariableStorageKey};
use std::collections::{HashMap, VecDeque};

#[derive(Clone, Debug)]
pub struct ScopedVariable {
    pub resolved_type: asg::Type,
    pub key: VariableStorageKey,
}

impl ScopedVariable {
    pub fn new(resolved_type: asg::Type, key: VariableStorageKey) -> Self {
        Self { resolved_type, key }
    }
}

#[derive(Clone, Debug)]
pub struct VariableHaystack {
    variables: VecDeque<HashMap<String, ScopedVariable>>,
}

impl VariableHaystack {
    pub fn new() -> Self {
        let mut variables = VecDeque::with_capacity(16);
        variables.push_front(HashMap::new());
        Self { variables }
    }

    pub fn find(&self, name: &str) -> Option<&ScopedVariable> {
        for variables in self.variables.iter() {
            if let Some(scoped_variable) = variables.get(name) {
                return Some(scoped_variable);
            }
        }

        None
    }

    pub fn put(
        &mut self,
        name: impl ToString,
        resolved_type: asg::Type,
        key: VariableStorageKey,
    ) {
        self.variables
            .front_mut()
            .expect("at least one scope")
            .insert(name.to_string(), ScopedVariable::new(resolved_type, key));
    }

    pub fn begin_scope(&mut self) {
        self.variables.push_front(Default::default());
    }

    pub fn end_scope(&mut self) {
        self.variables.pop_front().expect("scope to close");
    }
}
