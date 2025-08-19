use crate::{
    cfg::NodeRef,
    repr::{UnaliasedType, Variable, Variables},
};

// NOTE: We store a sorted array of NodeRefs that reference
// each node that declares a variable.
// Once type resolution is complete, unresolved variables will
// be pruned, and we will be left with a slot assignment and
// mapping for each variable.
// LIMITATION: A node cannot declare more that one variable.
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
    pub fn get(&self, node_ref: NodeRef) -> Option<&VariableTracker<'env>> {
        self.storage
            .binary_search_by(|item| item.declared_at.cmp(&node_ref))
            .ok()
            .map(|found| &self.storage[found])
    }

    pub fn get_mut(&mut self, node_ref: NodeRef) -> Option<&mut VariableTracker<'env>> {
        self.storage
            .binary_search_by(|item| item.declared_at.cmp(&node_ref))
            .ok()
            .map(|found| &mut self.storage[found])
    }

    pub fn assign_resolved_type(&mut self, node_ref: NodeRef, ty: UnaliasedType<'env>) {
        assert!(
            self.get_mut(node_ref)
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
    pub declared_at: NodeRef,
    // Resolved during type resolution step.
    pub ty: Option<UnaliasedType<'env>>,
}
