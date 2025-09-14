use crate::repr::UnaliasedType;
use arena::{Arena, Idx, new_id_with_niche};

new_id_with_niche!(VariableId, u32);
pub type VariableRef<'env> = Idx<VariableId, Variable<'env>>;

#[derive(Clone, Debug, Default)]
pub struct Variables<'env> {
    storage: Arena<VariableId, Variable<'env>>,
}

impl<'env> Variables<'env> {
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Variable<'env>> {
        self.storage.values()
    }

    pub fn get(&self, variable_ref: VariableRef<'env>) -> &Variable<'env> {
        &self.storage[variable_ref]
    }

    pub fn push(&mut self, variable: Variable<'env>) -> VariableRef<'env> {
        self.storage.alloc(variable)
    }
}

#[derive(Debug, Clone)]
pub struct Variable<'env> {
    pub ty: UnaliasedType<'env>,
}

impl<'env> Variable<'env> {
    pub fn new(ty: UnaliasedType<'env>) -> Self {
        Self { ty }
    }
}
