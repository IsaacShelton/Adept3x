use crate::c::preprocessor::ast::Define;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Environment {
    pub defines: HashMap<String, Define>,
}

impl Environment {
    pub fn add_define(&mut self, define: Define) {
        // NOTE: According to the C standard, macros are not supposed to be redefined unless they
        // are "identical", but most compilers allow redefining so we will follow suit.
        self.defines.insert(define.name.clone(), define);
    }

    pub fn find_define(&self, name: &str) -> Option<&Define> {
        self.defines.get(name)
    }

    pub fn remove_define(&mut self, name: &str) {
        self.defines.remove(name);
    }
}
