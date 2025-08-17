use crate::module_graph::{
    ModuleId, ModulePartHandle, ModuleRef,
    part::{ModulePartId, ModulePartRef},
};
use arena::ArenaMap;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default)]
pub struct WildcardImportsGraph<'env> {
    public: ArenaMap<ModuleId, Vec<Wildcard<'env>>>,
    protected: ArenaMap<ModuleId, Vec<Wildcard<'env>>>,
    private: HashMap<ModulePartHandle<'env>, Vec<Wildcard<'env>>>,

    // These are "public", but not within this module graph (as the module part is hidden here)
    // If the module part ever gets unhidden, then the wildcards for it will be merged
    // into the normal "public".
    hidden_public: HashMap<ModulePartHandle<'env>, Vec<Wildcard<'env>>>,

    // These are "protected", but not within this module graph (as the module part is hidden here)
    // If the module part ever gets unhidden, then the wildcards for it will be merged
    // into the normal "protected".
    hidden_protected: HashMap<ModulePartHandle<'env>, Vec<Wildcard<'env>>>,
}

impl<'env> WildcardImportsGraph<'env> {
    pub fn unhide(&mut self, handle: ModulePartHandle<'env>) -> bool {
        let mut did = false;

        if let Some(mut new_public) = self.hidden_public.remove(&handle) {
            if let Some(existing_public) = self.public.get_mut(handle.module_ref) {
                existing_public.append(&mut new_public);
            } else {
                self.public.insert(handle.module_ref, new_public);
            }
            did = true;
        }

        if let Some(mut new_protected) = self.hidden_protected.remove(&handle) {
            if let Some(existing_protected) = self.protected.get_mut(handle.module_ref) {
                existing_protected.append(&mut new_protected);
            } else {
                self.protected.insert(handle.module_ref, new_protected);
            }
            did = true;
        }

        did
    }

    pub fn compute_wildcards(&self, start: ModulePartHandle<'env>) -> Vec<ModuleRef<'env>> {
        let mut seen = HashSet::new();
        let mut stack = Vec::with_capacity(8);

        stack.push(start.module_ref);
        seen.insert(start.module_ref);

        for wildcard in self.private.get(&start).into_iter().flatten() {
            let module_ref = wildcard.module;

            if seen.insert(module_ref) {
                stack.push(module_ref);
            }
        }

        for wildcard in self.protected.get(start.module_ref).into_iter().flatten() {
            let module_ref = wildcard.module;

            if seen.insert(module_ref) {
                stack.push(module_ref);
            }
        }

        while let Some(working_module_ref) = stack.pop() {
            for wildcard in self.public.get(working_module_ref).into_iter().flatten() {
                let module_ref = wildcard.module;

                if seen.insert(module_ref) {
                    stack.push(module_ref);
                }
            }
        }

        seen.into_iter().collect()
    }
}

/// Corresponds to one wildcard import of a module
#[derive(Debug)]
pub struct Wildcard<'env> {
    module: ModuleRef<'env>,
    transforms: Vec<Transform>,
}

#[derive(Debug)]
pub struct Transform {}
