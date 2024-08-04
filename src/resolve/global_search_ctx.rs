use super::error::{ResolveError, ResolveErrorKind};
use crate::{
    resolved::{self, GlobalVarRef},
    source_files::Source,
};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct GlobalSearchCtx {
    globals: HashMap<String, (resolved::Type, GlobalVarRef)>,
}

impl GlobalSearchCtx {
    pub fn new() -> Self {
        Self {
            globals: Default::default(),
        }
    }

    pub fn find_global_or_error(
        &self,
        name: &str,
        source: Source,
    ) -> Result<(&resolved::Type, &GlobalVarRef), ResolveError> {
        match self.find_global(name) {
            Some(global) => Ok(global),
            None => Err(ResolveErrorKind::UndeclaredVariable {
                name: name.to_string(),
            }
            .at(source)),
        }
    }

    pub fn find_global(&self, name: &str) -> Option<(&resolved::Type, &GlobalVarRef)> {
        if let Some((resolved_type, key)) = self.globals.get(name) {
            return Some((resolved_type, key));
        }
        None
    }

    pub fn put(
        &mut self,
        name: impl ToString,
        resolved_type: resolved::Type,
        reference: GlobalVarRef,
    ) {
        self.globals
            .insert(name.to_string(), (resolved_type, reference));
    }
}
