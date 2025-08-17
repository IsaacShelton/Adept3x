use crate::{
    module_graph::{
        ModuleBreakOffMode, ModuleGraph, ModuleGraphMeta, ModuleGraphRef, ModuleGraphWeb,
        ModulePartHandle, ModulePartVisibility, Upserted,
    },
    repr::{Compiler, DeclHead},
};
use attributes::Privacy;
use by_address::ByAddress;
use derivative::Derivative;
use source_files::SourceFiles;
use std::path::Path;

#[derive(Copy, Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ModuleView<'env> {
    pub web: ByAddress<&'env ModuleGraphWeb<'env>>,
    pub graph: ModuleGraphRef,
    pub handle: ModulePartHandle<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub canonical_filename: &'env Path,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub canonical_module_filename: &'env Path,
}

impl<'env> ModuleView<'env> {
    pub fn new(
        web: &'env ModuleGraphWeb<'env>,
        graph: ModuleGraphRef,
        handle: ModulePartHandle<'env>,
        canonical_filename: &'env Path,
        canonical_module_filename: &'env Path,
    ) -> Self {
        Self {
            web: ByAddress(web),
            graph,
            handle,
            canonical_filename,
            canonical_module_filename,
        }
    }

    pub fn upsert_part(&self, canonical_filename: &'env Path) -> Upserted<ModuleView<'env>> {
        let new_part_ref = self.web.graph_mut(self.graph, |graph| {
            graph.modules.get_mut().unwrap().arena[self.handle.module_ref]
                .find_or_create_part(canonical_filename, ModulePartVisibility::Visible)
                .if_found(|found| {
                    graph.unhide_mut(ModulePartHandle::new(self.handle.module_ref, found))
                })
        });

        let is_found = new_part_ref.is_found();
        let view = ModuleView::new(
            &self.web,
            self.graph,
            ModulePartHandle::new(self.handle.module_ref, new_part_ref.out_of()),
            canonical_filename,
            self.canonical_module_filename,
        );

        if is_found {
            Upserted::Existing(view)
        } else {
            Upserted::Created(view)
        }
    }

    pub fn graph<Ret>(&self, f: impl FnOnce(&ModuleGraph<'env>) -> Ret) -> Ret {
        self.web.graph(self.graph, f)
    }

    pub fn break_off(
        &self,
        mode: ModuleBreakOffMode,
        canonical_filename: &'env Path,
        compiler: &Compiler,
    ) -> Upserted<Self> {
        match mode {
            ModuleBreakOffMode::Module => self
                .web
                .upsert_module_with_initial_part(self.graph, canonical_filename),
            ModuleBreakOffMode::Part => self.upsert_part(canonical_filename),
        }
    }
}
