use super::{Decl, DeclScopeRef, DeclSet};
use std::collections::{HashMap, hash_map::Entry};

/// A collection of identifiers mapped to declaration sets
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeclScope {
    parent: Option<DeclScopeRef>,
    names: HashMap<String, DeclSet>,
}

impl DeclScope {
    pub fn new() -> Self {
        Self {
            parent: None,
            names: Default::default(),
        }
    }

    pub fn get(&self, name: &str) -> Option<&DeclSet> {
        self.names.get(name).as_ref().copied()
    }

    pub fn entry(&mut self, name: String) -> Entry<String, DeclSet> {
        self.names.entry(name)
    }

    pub fn push_unique(&mut self, name: String, decl: Decl) {
        self.names.entry(name).or_default().push_unique(decl);
    }

    pub fn parent(&self) -> Option<DeclScopeRef> {
        self.parent
    }
}
