use super::Type;
use crate::cfg::NodeRef;

#[derive(Debug, Clone)]
pub struct Variables<'env> {
    storage: Box<[Variable<'env>]>,
}

impl<'env> Variables<'env> {
    pub fn len(&self) -> usize {
        self.storage.len()
    }

    pub fn get(&self, node_ref: NodeRef) -> Option<&Variable<'env>> {
        self.storage
            .binary_search_by(|item| item.declared_at.cmp(&node_ref))
            .ok()
            .map(|found| &self.storage[found])
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, &Variable<'env>)> {
        self.storage.iter().enumerate()
    }
}

impl<'env> FromIterator<Variable<'env>> for Variables<'env> {
    fn from_iter<T: IntoIterator<Item = Variable<'env>>>(iter: T) -> Self {
        Self {
            storage: iter.into_iter().collect(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Variable<'env> {
    pub declared_at: NodeRef,
    pub ty: &'env Type<'env>,
}
