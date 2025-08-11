#![allow(unused)]

mod link;
mod module;
mod part;
mod symbol_channel;
mod wildcard;

use crate::repr::{Compiler, DeclHead, DeclHeadSet, DeclSet, Type};
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
    path::Path,
    sync::{Mutex, RwLock},
};
use std_ext::HashMapExt;
pub use symbol_channel::*;
use target::Target;
pub use wildcard::*;

new_id_with_niche!(ModuleId, u32);
pub type ModuleRef<'env> = Idx<ModuleId, Module<'env>>;

new_id_with_niche!(ModuleGraphId, u16);
pub type ModuleGraphRef<'env> = Idx<ModuleGraphId, ModuleGraph<'env>>;

#[derive(Debug, Default)]
pub struct ModuleGraphWeb<'env> {
    graphs: LockFreeArena<ModuleGraphId, ModuleGraph<'env>>,
}

impl<'env> ModuleGraphWeb<'env> {
    pub fn add_module_graph(
        &self,
        comptime: Option<ModuleGraphRef<'env>>,
        meta: ModuleGraphMeta,
    ) -> ModuleGraphRef<'env> {
        self.graphs.alloc(ModuleGraph::new(comptime, meta))
    }

    pub fn add_module_with_initial_part(
        &'env self,
        module_graph: ModuleGraphRef<'env>,
    ) -> ModuleView<'env> {
        let handle = { self.graphs[module_graph].add_module_with_initial_part() };

        ModuleView {
            web: ByAddress(self),
            graph: module_graph,
            handle,
        }
    }

    pub fn graph(&self, module_graph: ModuleGraphRef<'env>) -> &ModuleGraph<'env> {
        &self.graphs[module_graph]
    }

    pub fn meta(&self, module_graph: ModuleGraphRef<'env>) -> &ModuleGraphMeta {
        &self.graphs[module_graph].meta
    }
}

#[derive(Debug)]
pub struct ModuleGraphMeta {
    // Human-readable title for this module graph.
    pub title: &'static str,

    // Whether this module graph is meant for compile-time code evaluation.
    pub is_comptime: Option<ComptimeKind>,

    // The target for this module graph
    pub target: Target,
}

#[derive(Debug)]
pub enum ComptimeKind {
    Sandbox,
}

#[derive(Debug)]
pub struct ModuleGraph<'env> {
    // `None` means self-reference (the module graph is its own comptime).
    // If `Some` the referenced module must self-reference.
    comptime: Option<ModuleGraphRef<'env>>,

    // Each of the modules within this module graph
    modules: Mutex<Arena<ModuleId, Module<'env>>>,

    // Each of the wildcard imports within this module graph
    wildcard_imports: Mutex<WildcardImportsGraph<'env>>,

    // Each of the links that must stay consistent when adding symbols
    consistency: AppendOnlyVec<Link<'env>>,

    // Metadata about the purpose of this module graph
    meta: ModuleGraphMeta,
}

impl<'env> ModuleGraph<'env> {
    pub fn new(comptime: Option<ModuleGraphRef<'env>>, meta: ModuleGraphMeta) -> Self {
        Self {
            comptime: comptime,
            modules: Default::default(),
            wildcard_imports: Default::default(),
            consistency: Default::default(),
            meta,
        }
    }

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

#[derive(Clone, Debug)]
pub enum ModuleBreakOffMode {
    Module,
    Part,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ModuleView<'env> {
    pub web: ByAddress<&'env ModuleGraphWeb<'env>>,
    pub graph: ModuleGraphRef<'env>,
    pub handle: ModulePartHandle<'env>,
}

impl<'env> ModuleView<'env> {
    pub fn new(
        web: &'env ModuleGraphWeb<'env>,
        graph: ModuleGraphRef<'env>,
        handle: ModulePartHandle<'env>,
    ) -> Self {
        Self {
            web: ByAddress(web),
            graph,
            handle,
        }
    }

    pub fn meta(&self) -> &ModuleGraphMeta {
        self.web.meta(self.graph)
    }

    pub fn graph(&self) -> &ModuleGraph<'env> {
        self.web.graph(self.graph)
    }

    pub fn comptime_graph(&self) -> Option<&ModuleGraph<'env>> {
        self.web
            .graph(self.graph)
            .comptime
            .map(|comptime| self.web.graph(comptime))
    }

    pub fn break_off(
        &self,
        mode: ModuleBreakOffMode,
        canonical_filename: &Path,
        compiler: &Compiler,
    ) -> Self {
        match mode {
            ModuleBreakOffMode::Module => self.break_off_into_module(canonical_filename, compiler),
            ModuleBreakOffMode::Part => self.break_off_into_part(canonical_filename, compiler),
        }
    }

    pub fn break_off_into_module(&self, canonical_filename: &Path, compiler: &Compiler) -> Self {
        todo!(
            "break_off_into_module {:?}",
            compiler.filename(canonical_filename)
        )

        /*
        Self::new(
            &self.module_graph_web,
            self.module_graph,
            self.module_graph_web.add_module_with_initial_part(),
        )
        */
    }

    pub fn break_off_into_part(&self, canonical_filename: &Path, compiler: &Compiler) -> Self {
        {
            let graph = self.graph();

            if let Some(comptime) = self.comptime_graph() {
                // We are "runtime" relative to another module graph,
                // we need to update our comptime module graph to
                // include the new part.
                // If the part does not exist in the comptime module,
                // we need to make it hidden so it doesn't influence
                // anything there.
                todo!(
                    "break_off_into_part (has comptime) {:?}",
                    compiler.filename(canonical_filename),
                )
            } else {
                // This module graph is its own "comptime". If the part we want
                // already exists as a hidden part, we need to use
                // that and update everyone (in this module graph)
                // who may depend on the new symbols.
                todo!(
                    "break_off_into_part (is self comptime) {:?}",
                    compiler.filename(canonical_filename),
                )
            }
        }

        todo!("break_off_into_part")
        // Self::new(&self.module_graph, self.module_graph.add_part(self.handle))
    }
}
