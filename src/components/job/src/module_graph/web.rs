use crate::{
    ExecutionCtx,
    module_graph::{
        FoundOrCreated, ModuleGraph, ModuleGraphRef, ModulePartHandle, ModuleRef, Upserted,
        meta::ModuleGraphMeta, module_graph_ref::ComptimeKind, view::ModuleView,
        web_inner::ModuleGraphWebInner,
    },
    repr::DeclHead,
};
use attributes::Privacy;
use std::{path::Path, sync::RwLock};
use target::Target;

#[derive(Debug)]
pub struct ModuleGraphWeb<'env> {
    inner: RwLock<ModuleGraphWebInner<'env>>,
}

impl<'env> ModuleGraphWeb<'env> {
    pub fn new(target: Target, ctx: &mut ExecutionCtx<'env>) -> Self {
        Self {
            inner: RwLock::new(ModuleGraphWebInner::new(target, ctx)),
        }
    }

    pub fn graph<Ret>(
        &self,
        graph_ref: ModuleGraphRef,
        f: impl FnOnce(&ModuleGraph<'env>) -> Ret,
    ) -> Ret {
        f(self.inner.read().unwrap().graph(graph_ref))
    }

    pub fn graph_mut<Ret>(
        &self,
        graph_ref: ModuleGraphRef,
        f: impl FnOnce(&mut ModuleGraph<'env>) -> Ret,
    ) -> Ret {
        f(self.inner.write().unwrap().graph_mut(graph_ref))
    }

    pub fn upsert_module_with_initial_part(
        &'env self,
        graph_ref: ModuleGraphRef,
        canonical_filename: &'env Path,
    ) -> Upserted<ModuleView<'env>> {
        let handle = self
            .inner
            .write()
            .unwrap()
            .find_or_create_module_with_initial_part(graph_ref, canonical_filename);

        let is_found = handle.is_found();
        let view = ModuleView::new(
            self,
            graph_ref,
            handle.out_of(),
            canonical_filename,
            canonical_filename,
        );

        if is_found {
            Upserted::Existing(view)
        } else {
            Upserted::Created(view)
        }
    }

    pub fn add_symbol(
        &self,
        graph_ref: ModuleGraphRef,
        handle: ModulePartHandle<'env>,
        privacy: Privacy,
        name: &'env str,
        decl_head: DeclHead<'env>,
    ) {
        self.inner
            .read()
            .unwrap()
            .add_symbol(graph_ref, handle, privacy, name, decl_head);
    }
}
