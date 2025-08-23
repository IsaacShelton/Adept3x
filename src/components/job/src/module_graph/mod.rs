#![allow(unused)]

mod link;
mod meta;
mod module;
mod module_graph_ref;
mod part;
mod symbol_channel;
mod view;
mod web;
mod web_inner;
mod wildcard;

use crate::{
    ExecutionCtx, Ir,
    repr::{Compiler, DeclHead, DeclHeadSet, Type},
};
use append_only_vec::AppendOnlyVec;
use arena::{Arena, ArenaMap, Idx, LockFreeArena, new_id_with_niche};
use attributes::Privacy;
use by_address::ByAddress;
use derive_more::IsVariant;
pub use link::*;
pub use meta::ModuleGraphMeta;
pub use module::*;
pub use module_graph_ref::{ComptimeKind, ModuleGraphRef};
use num_traits::bounds::LowerBounded;
pub use part::*;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Mutex, RwLock},
};
use std_ext::HashMapExt;
pub use symbol_channel::*;
use target::Target;
pub use view::ModuleView;
pub use web::ModuleGraphWeb;
pub use wildcard::*;

new_id_with_niche!(ModuleId, u32);
pub type ModuleRef<'env> = Idx<ModuleId, Module<'env>>;

#[derive(Debug, Default)]
pub struct Modules<'env> {
    // The modules themselves
    arena: Arena<ModuleId, Module<'env>>,
    // The initial file for each module
    filenames: HashMap<PathBuf, ModulePartHandle<'env>>,
}

impl<'env> Modules<'env> {
    pub fn find_or_create_module_with_initial_part(
        &mut self,
        canonical_filename: &Path,
        preferred_visibility: ModulePartVisibility,
    ) -> FoundOrCreated<ModulePartHandle<'env>> {
        if let Some(existing) = self.filenames.get(canonical_filename) {
            return FoundOrCreated::Found(*existing);
        }

        let created_module = self.arena.alloc(Module::default());
        let created_part = self.arena[created_module]
            .find_or_create_part(canonical_filename, preferred_visibility)
            .out_of();

        let created = ModulePartHandle::new(created_module, created_part);
        self.filenames.insert(canonical_filename.into(), created);
        FoundOrCreated::Created(created)
    }
}

#[derive(Debug)]
pub struct ModuleGraph<'env> {
    // Each of the modules within this module graph
    modules: Mutex<Modules<'env>>,

    // Each of the wildcard imports within this module graph
    // NOTE: Must be acquired after `modules` mutex if requiring both
    wildcard_imports: Mutex<WildcardImportsGraph<'env>>,

    // Each of the links that must stay consistent when adding symbols
    consistency: AppendOnlyVec<Link<'env>>,

    // Metadata about the purpose of this module graph
    meta: ModuleGraphMeta,

    // IR module
    pub ir: &'env Ir<'env>,
}

#[derive(IsVariant)]
pub enum FoundOrCreated<T> {
    Found(T),
    Created(T),
}

impl<T: Copy> FoundOrCreated<T> {
    pub fn out_of(self) -> T {
        match self {
            Self::Found(found) => found,
            Self::Created(created) => created,
        }
    }

    pub fn if_found<U>(self, f: impl FnOnce(T) -> U) -> Self {
        if let Self::Found(found) = self {
            f(found);
        }

        self
    }
}

#[derive(IsVariant)]
pub enum Upserted<T> {
    Existing(T),
    Created(T),
}

impl<T: Copy> Upserted<T> {
    pub fn out_of(self) -> T {
        match self {
            Self::Existing(existing) => existing,
            Self::Created(created) => created,
        }
    }
}

impl<'env> ModuleGraph<'env> {
    pub fn new(meta: ModuleGraphMeta, ctx: &mut ExecutionCtx<'env>) -> Self {
        Self {
            modules: Default::default(),
            wildcard_imports: Default::default(),
            consistency: Default::default(),
            meta,
            ir: ctx.alloc(Ir::default()),
        }
    }

    pub fn find_or_create_module(
        &self,
        canonical_filename: &Path,
        preferred_creation_visibility: ModulePartVisibility,
    ) -> FoundOrCreated<ModulePartHandle<'env>> {
        let mut modules = self.modules.lock().unwrap();

        if let Some(found) = modules.filenames.get(canonical_filename) {
            return FoundOrCreated::Found(*found);
        }

        let created_module = modules.arena.alloc(Module::default());
        let created_part = modules.arena[created_module]
            .find_or_create_part(canonical_filename, preferred_creation_visibility)
            .if_found(|found| self.unhide_via_locks(ModulePartHandle::new(created_module, found)))
            .out_of();

        let created = ModulePartHandle::new(created_module, created_part);
        modules.filenames.insert(canonical_filename.into(), created);

        return FoundOrCreated::Created(created);
    }

    pub fn add_symbol(
        &self,
        handle: ModulePartHandle<'env>,
        privacy: Privacy,
        name: &'env str,
        decl_head: DeclHead<'env>,
    ) {
        let module = &mut self.modules.lock().unwrap().arena[handle.module_ref];
        module.add_symbol(handle.part_ref, privacy, name, decl_head);
    }

    pub fn unhide_via_locks(&self, handle: ModulePartHandle<'env>) -> bool {
        let mut modules = self.modules.lock().unwrap();
        let module = &mut modules.arena[handle.module_ref];

        if let Some(hidden) = module.get_mut(handle.part_ref).unhide() {
            module.add_previously_hidden(hidden);
            self.wildcard_imports.lock().unwrap().unhide(handle);
            true
        } else {
            false
        }
    }

    pub fn unhide_mut(&mut self, handle: ModulePartHandle<'env>) -> bool {
        let mut modules = self.modules.get_mut().unwrap();
        let module = &mut modules.arena[handle.module_ref];

        if let Some(hidden) = module.get_mut(handle.part_ref).unhide() {
            module.add_previously_hidden(hidden);
            self.wildcard_imports.lock().unwrap().unhide(handle);
            true
        } else {
            false
        }
    }

    pub fn meta(&self) -> &ModuleGraphMeta {
        &self.meta
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModulePartHandle<'env> {
    pub module_ref: ModuleRef<'env>,
    pub part_ref: ModulePartRef<'env>,
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
pub enum ModuleBreakOffMode {
    Module,
    Part,
}
