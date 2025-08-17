use crate::{
    module_graph::{
        ModuleBreakOffMode, ModuleGraph, ModuleGraphMeta, ModuleGraphRef, ModuleGraphWeb,
        ModulePartHandle,
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
}

impl<'env> ModuleView<'env> {
    pub fn new(
        web: &'env ModuleGraphWeb<'env>,
        graph: ModuleGraphRef,
        handle: ModulePartHandle<'env>,
        canonical_filename: &'env Path,
    ) -> Self {
        Self {
            web: ByAddress(web),
            graph,
            handle,
            canonical_filename,
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
    ) -> Self {
        match mode {
            ModuleBreakOffMode::Module => self
                .web
                .find_or_create_module_with_initial_part(self.graph, canonical_filename)
                .out_of(),
            ModuleBreakOffMode::Part => {
                self.web
                    .find_or_create_part(self.graph, self.handle.module_ref, canonical_filename)
            }
        }
    }
}
