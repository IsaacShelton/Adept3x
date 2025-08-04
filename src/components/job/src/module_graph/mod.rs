#![allow(unused)]

mod link;
mod module;
mod part;
mod symbol_channel;
mod wildcard;

use crate::repr::{DeclHead, DeclHeadSet, DeclSet, Type};
use append_only_vec::AppendOnlyVec;
use arena::{Arena, ArenaMap, Idx, LockFreeArena, new_id_with_niche};
use attributes::Privacy;
use by_address::ByAddress;
pub use link::*;
pub use module::*;
use num_traits::bounds::LowerBounded;
pub use part::*;
use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};
use std_ext::HashMapExt;
pub use symbol_channel::*;
pub use wildcard::*;

new_id_with_niche!(ModuleId, u32);
pub type ModuleRef<'env> = Idx<ModuleId, Module<'env>>;

#[derive(Debug, Default)]
pub struct ModuleGraph<'env> {
    modules: Mutex<Arena<ModuleId, Module<'env>>>,
    wildcard_imports: Mutex<WildcardImportsGraph<'env>>,
    consistency: AppendOnlyVec<Link<'env>>,
}

impl<'env> ModuleGraph<'env> {
    pub fn add_module_with_initial_part(&self) -> ModulePartHandle<'env> {
        let mut module = Module::default();
        let part_ref = module.add_part();
        let module_ref = self.modules.lock().unwrap().alloc(module);
        ModulePartHandle::new(module_ref, part_ref)
    }

    pub fn detect_broken_links(&self) -> impl Iterator<Item = &Link<'env>> {
        self.consistency.iter().filter(|link| {
            match self.lookup_symbol_inner(link.name, link.handle, link.constraints.clone(), false)
            {
                Err(LookupError::Ambiguous) => true,
                _ => false,
            }
        })
    }

    pub fn add_symbol(
        &self,
        privacy: Privacy,
        name: &'env str,
        decl_head: DeclHead<'env>,
        handle: ModulePartHandle<'env>,
    ) {
        let module = &mut self.modules.lock().unwrap()[handle.module_ref];
        module.add_symbol(privacy, name, decl_head, handle.part_ref);
    }

    pub fn lookup_symbol(
        &self,
        name: &'env str,
        handle: ModulePartHandle<'env>,
        constraints: LookupConstraints<'env>,
    ) -> Result<DeclHead<'env>, LookupError> {
        self.lookup_symbol_inner(name, handle, constraints, true)
    }

    pub fn lookup_symbol_inner(
        &self,
        name: &'env str,
        handle: ModulePartHandle<'env>,
        constraints: LookupConstraints<'env>,
        add_link: bool,
    ) -> Result<DeclHead<'env>, LookupError> {
        let wildcards = self
            .wildcard_imports
            .lock()
            .unwrap()
            .compute_wildcards(handle);

        let modules = self.modules.lock().unwrap();
        let module = &modules[handle.module_ref];

        let mut found = None;

        for head in module.iter_symbols(name, handle.part_ref) {
            if constraints.is_match(head) {
                if found.is_some() {
                    return Err(LookupError::Ambiguous);
                } else {
                    found = Some(*head);
                }
            }
        }

        for wildcard in wildcards {
            for head in modules[wildcard].iter_public_symbols(name) {
                if constraints.is_match(head) {
                    if found.is_some() {
                        return Err(LookupError::Ambiguous);
                    } else {
                        found = Some(*head);
                    }
                }
            }
        }

        if let Some(found) = found {
            if add_link {
                self.consistency
                    .push(Link::new(name, handle, constraints, found));
            }

            Ok(found)
        } else {
            Err(LookupError::NotFound)
        }
    }

    pub fn add_part(&self, existing: ModulePartHandle<'env>) -> ModulePartHandle<'env> {
        let part_ref = self.modules.lock().unwrap()[existing.module_ref].add_part();
        ModulePartHandle::new(existing.module_ref, part_ref)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModulePartHandle<'env> {
    module_ref: ModuleRef<'env>,
    part_ref: ModulePartRef<'env>,
}

impl<'env> ModulePartHandle<'env> {
    pub fn new(module_ref: ModuleRef<'env>, part_ref: ModulePartRef<'env>) -> Self {
        Self {
            module_ref,
            part_ref,
        }
    }
}

#[derive(Clone, Debug)]
pub enum LookupError {
    NotFound,
    Ambiguous,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModuleView<'env> {
    pub module_graph: ByAddress<&'env ModuleGraph<'env>>,
    pub handle: ModulePartHandle<'env>,
}
