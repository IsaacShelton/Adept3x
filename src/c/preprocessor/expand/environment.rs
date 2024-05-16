use crate::c::preprocessor::ast::Define;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Environment {
    pub defines: HashMap<String, Define>,
}

impl Environment {
    pub fn add_define(&mut self, define: Define) {
        self.defines
            .insert(define.name.clone(), define);
    }

    pub fn find_define(&self, name: &str) -> Option<&Define> {
        // NOTE: The major C compilers don't allow defining both an object-like
        // and a function-like macro of the same name at the same time,
        // so we will follow suite, although this violates the standard.
        self.defines.get(name)
    }

    pub fn remove_define(&mut self, name: &str) {
        self.defines.remove(name);
    }
}
