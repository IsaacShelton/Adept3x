use crate::c::preprocessor::ast::{Define, DefineKind};
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
pub struct Environment {
    pub defines: HashMap<String, Vec<Define>>,
}

impl Environment {
    pub fn add_define(&mut self, define: &Define) {
        if let Some(existing) = self.defines.get_mut(&define.name) {
            for (i, old_define) in existing.iter().enumerate() {
                if define.overwrites(old_define) {
                    existing.remove(i);
                    existing.push(define.clone());
                    return;
                }
            }
        }

        self.defines
            .insert(define.name.clone(), vec![define.clone()]);
    }

    pub fn find_define(&self, name: &str, arity: Option<usize>) -> Option<&Define> {
        for define in self.defines.get(name).into_iter().flatten() {
            let is_match = match &define.kind {
                DefineKind::Normal(_) => arity.is_none(),
                DefineKind::Macro(m) => arity.map_or(false, |arity| {
                    arity == m.parameters.len() || (arity > m.parameters.len() && m.is_variadic)
                }),
            };

            if is_match {
                return Some(define);
            }
        }

        None
    }

    pub fn find_defines_of_name(&self, name: &str) -> Option<&Vec<Define>> {
        self.defines.get(name)
    }

    pub fn remove_define(&mut self, name: &str) {
        self.defines.remove(name);
    }
}
