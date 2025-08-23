use crate::{
    ExecutionCtx,
    module_graph::{
        ComptimeKind, FoundOrCreated, ModuleGraph, ModuleGraphRef, ModulePartHandle,
        ModulePartVisibility, ModuleRef, meta::ModuleGraphMeta,
    },
    repr::DeclHead,
};
use arena::LockFreeArena;
use attributes::Privacy;
use std::path::Path;
use target::Target;

#[derive(Debug)]
pub struct ModuleGraphWebInner<'env> {
    runtime: ModuleGraph<'env>,
    comptime_sandbox: ModuleGraph<'env>,
    comptime_target: ModuleGraph<'env>,
    comptime_host: ModuleGraph<'env>,
}

impl<'env> ModuleGraphWebInner<'env> {
    pub fn new(target: Target, ctx: &mut ExecutionCtx<'env>) -> Self {
        Self {
            runtime: ModuleGraph::new(
                ModuleGraphMeta {
                    title: "runtime",
                    self_ref: ModuleGraphRef::Runtime,
                    target,
                },
                ctx,
            ),
            comptime_sandbox: ModuleGraph::new(
                ModuleGraphMeta {
                    title: "sandbox",
                    self_ref: ModuleGraphRef::Comptime(ComptimeKind::Sandbox),
                    target: Target::SANDBOX,
                },
                ctx,
            ),
            comptime_target: ModuleGraph::new(
                ModuleGraphMeta {
                    title: "target",
                    self_ref: ModuleGraphRef::Comptime(ComptimeKind::Target),
                    target,
                },
                ctx,
            ),
            comptime_host: ModuleGraph::new(
                ModuleGraphMeta {
                    title: "host",
                    self_ref: ModuleGraphRef::Comptime(ComptimeKind::Host),
                    target: Target::HOST,
                },
                ctx,
            ),
        }
    }

    pub fn graph(&self, graph_ref: ModuleGraphRef) -> &ModuleGraph<'env> {
        match graph_ref {
            ModuleGraphRef::Runtime => &self.runtime,
            ModuleGraphRef::Comptime(comptime_kind) => match comptime_kind {
                ComptimeKind::Sandbox => &self.comptime_sandbox,
                ComptimeKind::Target => &self.comptime_target,
                ComptimeKind::Host => &self.comptime_host,
            },
        }
    }

    pub fn graph_mut(&mut self, graph_ref: ModuleGraphRef) -> &mut ModuleGraph<'env> {
        match graph_ref {
            ModuleGraphRef::Runtime => &mut self.runtime,
            ModuleGraphRef::Comptime(comptime_kind) => match comptime_kind {
                ComptimeKind::Sandbox => &mut self.comptime_sandbox,
                ComptimeKind::Target => &mut self.comptime_target,
                ComptimeKind::Host => &mut self.comptime_host,
            },
        }
    }

    pub fn find_or_create_module_with_initial_part(
        &mut self,
        graph_ref: ModuleGraphRef,
        canonical_filename: &Path,
    ) -> FoundOrCreated<ModulePartHandle<'env>> {
        let graph = self.graph_mut(graph_ref);

        graph
            .modules
            .get_mut()
            .unwrap()
            .find_or_create_module_with_initial_part(
                canonical_filename,
                ModulePartVisibility::Visible,
            )
            .if_found(|found| graph.unhide_mut(found))
    }

    pub fn add_symbol(
        &self,
        graph_ref: ModuleGraphRef,
        handle: ModulePartHandle<'env>,
        privacy: Privacy,
        name: &'env str,
        decl_head: DeclHead<'env>,
    ) {
        self.graph(graph_ref)
            .add_symbol(handle, privacy, name, decl_head);
    }
}
