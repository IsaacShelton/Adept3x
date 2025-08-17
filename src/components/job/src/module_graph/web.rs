use crate::module_graph::{
    FoundOrCreated, ModuleGraph, ModuleGraphRef, ModulePartHandle, ModuleRef,
    meta::ModuleGraphMeta, module_graph_ref::ComptimeKind, view::ModuleView,
    web_inner::ModuleGraphWebInner,
};
use std::{path::Path, sync::RwLock};
use target::Target;

#[derive(Debug)]
pub struct ModuleGraphWeb<'env> {
    inner: RwLock<ModuleGraphWebInner<'env>>,
}

impl<'env> ModuleGraphWeb<'env> {
    pub fn new(target: Target) -> Self {
        Self {
            inner: RwLock::new(ModuleGraphWebInner::new(target)),
        }
    }

    pub fn graph<Ret>(
        &self,
        graph_ref: ModuleGraphRef,
        f: impl FnOnce(&ModuleGraph<'env>) -> Ret,
    ) -> Ret {
        f(self.inner.read().unwrap().graph(graph_ref))
    }

    pub fn find_or_create_module_with_initial_part(
        &'env self,
        graph_ref: ModuleGraphRef,
        canonical_filename: &'env Path,
    ) -> FoundOrCreated<ModuleView<'env>> {
        let handle = self
            .inner
            .write()
            .unwrap()
            .find_or_create_module_with_initial_part(graph_ref, canonical_filename);

        let is_found = handle.is_found();
        let view = ModuleView::new(self, graph_ref, handle.out_of(), canonical_filename);

        if is_found {
            FoundOrCreated::Found(view)
        } else {
            FoundOrCreated::Created(view)
        }
    }

    pub fn find_or_create_part(
        &'env self,
        graph_ref: ModuleGraphRef,
        module_ref: ModuleRef<'env>,
        canonical_filename: &'env Path,
    ) -> ModuleView<'env> {
        ModuleView::new(
            self,
            graph_ref,
            self.inner.write().unwrap().find_or_create_part(
                graph_ref,
                module_ref,
                canonical_filename,
            ),
            canonical_filename,
        )
    }
}
