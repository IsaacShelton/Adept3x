use crate::{name::ResolvedName, resolved};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FunctionSearchCtx {
    pub available: HashMap<ResolvedName, Vec<resolved::FunctionRef>>,
}

impl FunctionSearchCtx {
    pub fn new() -> Self {
        Self {
            available: Default::default(),
        }
    }

    pub fn find_function(&self, name: &ResolvedName) -> Option<resolved::FunctionRef> {
        self.available
            .get(name)
            .and_then(|list| list.first())
            .copied()
    }
}
