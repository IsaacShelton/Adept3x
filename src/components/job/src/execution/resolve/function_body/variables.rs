#![allow(unused)]

use crate::{
    InstrRef,
    repr::{UnaliasedType, Variable, Variables},
};

// NOTE: We store a sorted array of InstrRefs that reference
// each instruction that declares a variable.
// Once type resolution is complete, unresolved variables will
// be pruned, and we will be left with a slot assignment and
// mapping for each variable.
// LIMITATION: An instruction cannot declare more that one variable.
#[derive(Debug, Clone, Default)]
pub struct VariableTrackers<'env> {
    storage: Box<[VariableTracker<'env>]>,
}

impl<'env> FromIterator<VariableTracker<'env>> for VariableTrackers<'env> {
    fn from_iter<T: IntoIterator<Item = VariableTracker<'env>>>(iter: T) -> Self {
        Self {
            storage: iter.into_iter().collect(),
        }
    }
}

impl<'env> VariableTrackers<'env> {
    pub fn get(&self, instr_ref: InstrRef) -> Option<&VariableTracker<'env>> {
        self.storage
            .binary_search_by(|item| item.declared_at.cmp(&instr_ref))
            .ok()
            .map(|found| &self.storage[found])
    }

    pub fn get_mut(&mut self, instr_ref: InstrRef) -> Option<&mut VariableTracker<'env>> {
        self.storage
            .binary_search_by(|item| item.declared_at.cmp(&instr_ref))
            .ok()
            .map(|found| &mut self.storage[found])
    }

    pub fn assign_resolved_type(&mut self, instr_ref: InstrRef, ty: UnaliasedType<'env>) {
        assert!(
            self.get_mut(instr_ref)
                .expect("variable to be tracked")
                .ty
                .replace(ty)
                .is_none()
        );
    }

    pub fn prune(self) -> Variables<'env> {
        self.storage
            .into_iter()
            .filter_map(|variable| {
                variable.ty.map(|ty| Variable {
                    declared_at: variable.declared_at,
                    ty,
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub struct VariableTracker<'env> {
    pub declared_at: InstrRef,
    // Resolved during type resolution step.
    pub ty: Option<UnaliasedType<'env>>,
}
