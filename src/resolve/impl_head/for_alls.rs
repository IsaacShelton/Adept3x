use crate::{resolve::error::ResolveError, source_files::Source};
use indexmap::IndexSet;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct ForAlls {
    substitution_polys: IndexSet<String>,
    trait_to_impl: HashMap<String, String>,
    impl_to_trait: HashMap<String, String>,
}

impl ForAlls {
    pub fn new(substitution_polys: IndexSet<String>) -> Self {
        Self {
            substitution_polys,
            trait_to_impl: Default::default(),
            impl_to_trait: Default::default(),
        }
    }

    pub fn insert(
        &mut self,
        in_trait: String,
        in_impl: String,
        source: Source,
    ) -> Result<(), ResolveError> {
        if self.substitution_polys.contains(&in_impl) {
            return Err(ResolveError::other("Inconsistent mapping", source));
        }

        if let Some(expected) = self.trait_to_impl.get(&in_trait) {
            if *expected != in_impl {
                return Err(ResolveError::other("Inconsistent mapping", source));
            }
        }

        if let Some(expected) = self.impl_to_trait.get(&in_impl) {
            if *expected != in_trait {
                return Err(ResolveError::other("Inconsistent mapping", source));
            }
        }

        if self.trait_to_impl.contains_key(&in_trait) && self.impl_to_trait.contains_key(&in_impl) {
            // Already exists, and is correct
            return Ok(());
        }

        if !self
            .trait_to_impl
            .insert(in_trait.clone(), in_impl.clone())
            .is_none()
            || !self.impl_to_trait.insert(in_impl, in_trait).is_none()
        {
            return Err(ResolveError::other("Inconsistent mapping", source));
        }

        Ok(())
    }
}
