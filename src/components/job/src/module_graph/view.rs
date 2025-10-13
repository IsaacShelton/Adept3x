use crate::{
    Continuation, Execution, Executor, Search, execution,
    module_graph::{
        ModuleBreakOffMode, ModuleGraph, ModuleGraphMeta, ModuleGraphRef, ModuleGraphWeb,
        ModulePartHandle, ModulePartVisibility, Upserted,
    },
    repr::{Compiler, DeclHead},
};
use attributes::Privacy;
use by_address::ByAddress;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;
use source_files::SourceFiles;
use std::path::Path;
use target::Target;

#[derive(Copy, Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ModuleView<'env> {
    #[derivative(Debug = "ignore")]
    pub web: ByAddress<&'env ModuleGraphWeb<'env>>,

    pub graph: ModuleGraphRef,
    pub handle: ModulePartHandle<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub meta: &'env ModuleGraphMeta,

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
        meta: &'env ModuleGraphMeta,
    ) -> Self {
        Self {
            web: ByAddress(web),
            graph,
            handle,
            canonical_filename,
            canonical_module_filename,
            meta,
        }
    }

    pub fn target(&self) -> &'env Target {
        &self.meta.target
    }

    #[must_use]
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
            self.meta,
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

    #[must_use]
    pub fn break_off(
        &self,
        mode: ModuleBreakOffMode,
        canonical_filename: &'env Path,
        compiler: &Compiler,
    ) -> Upserted<Self> {
        match mode {
            ModuleBreakOffMode::Module => {
                self.web
                    .upsert_module_with_initial_part(self.graph, self.meta, canonical_filename)
            }
            ModuleBreakOffMode::Part => self.upsert_part(canonical_filename),
        }
    }

    pub fn add_symbol(&self, privacy: Privacy, name: &'env str, decl_head: DeclHead<'env>) {
        self.web
            .add_symbol(self.graph, self.handle, privacy, name, decl_head);
    }

    pub fn find_symbol<'a, S>(
        &self,
        executor: &'a Executor<'env>,
        search: S,
    ) -> Result<DeclHead<'env>, impl FnOnce(Execution<'env>) -> Continuation<'env> + use<'env, S>>
    where
        S: Into<Search<'env>>,
    {
        let search = search.into();
        let graph = self.graph;
        let handle = self.handle;

        let searched_version = executor.pending_searches.get_or_default(graph, |graph| {
            graph.get_pending_search_version(search.name())
        });

        // TODO: ... Perform search ...

        match &search {
            Search::Func(func_search) => {
                let found = self.web.graph(graph, |graph| {
                    let module = graph.modules.lock().unwrap();

                    module.arena[handle.module_ref]
                        .iter_symbols(func_search.name, handle.part_ref)
                        .filter(|symbol| symbol.is_func_like())
                        .next()
                });

                if let Some(found) = found {
                    return Ok(found);
                }
            }
            Search::Namespace(namespace_search) => {
                let found = self.web.graph(graph, |graph| {
                    let module = graph.modules.lock().unwrap();

                    module.arena[handle.module_ref]
                        .iter_symbols(namespace_search.name, handle.part_ref)
                        .filter(|symbol| symbol.is_value_like())
                        .next()
                });

                if let Some(found) = found {
                    return Ok(found);
                }
            }
            Search::Type(type_search) => {
                let found = self.web.graph(graph, |graph| {
                    let module = graph.modules.lock().unwrap();

                    module.arena[handle.module_ref]
                        .iter_symbols(type_search.name, handle.part_ref)
                        .filter(|symbol| symbol.is_type_like())
                        .next()
                });

                if let Some(found) = found {
                    return Ok(found);
                }
            }
        }

        Err(move |execution| {
            Continuation::PendingSearch(execution, graph, searched_version, search)
        })
    }
}
