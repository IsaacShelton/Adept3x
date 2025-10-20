use crate::{
    Continuation, Execution, ExecutionCtx, Executor, NamespaceSearch,
    module_graph::{ModulePartHandle, ModulePartId, ModulePartRef, ModuleView},
    repr::{DeclHead, ValueLikeRef},
    sub_task::SubTask,
};
use arena::Id;
use diagnostics::ErrorDiagnostic;
use source_files::Source;

#[derive(Clone)]
pub struct ResolveNamespace<'env> {
    index: usize,
    view: ModuleView<'env>,
    source: Source,
}

impl<'env> ResolveNamespace<'env> {
    pub fn new(view: &'env ModuleView<'env>, source: Source) -> Self {
        Self {
            index: 0,
            view: *view,
            source,
        }
    }
}

impl<'env> SubTask<'env> for ResolveNamespace<'env> {
    type SubArtifact<'a>
        = ModuleView<'env>
    where
        Self: 'a,
        'env: 'a;

    type UserData<'a>
        = &'a [Box<str>]
    where
        Self: 'a,
        'env: 'a;

    fn execute_sub_task<'a, 'ctx>(
        &'a mut self,
        executor: &'a Executor<'env>,
        ctx: &'ctx mut ExecutionCtx<'env>,
        namespaces: Self::UserData<'a>,
    ) -> Result<
        Self::SubArtifact<'a>,
        Result<impl FnOnce(Execution<'env>) -> Continuation<'env> + 'env, ErrorDiagnostic>,
    > {
        while self.index < namespaces.len() {
            // TODO: Don't allocate this on every resume, or rather, ideally at all.
            // Once `NamePath` is parameterized by 'env, this shouldn't be necessary anymore,
            // but since transmuting from `&'env [Box<str>]` to `&'env [&'env str]` can't be
            // assumed to be valid, we're stuck allocating for now...
            let name = ctx.alloc_str(&namespaces[self.index]);

            let decl_head = self
                .view
                .find_symbol(
                    executor,
                    NamespaceSearch {
                        name,
                        source: self.source,
                    },
                )
                .map_err(Ok)?;

            match decl_head {
                DeclHead::ValueLike(ValueLikeRef::Namespace(module_ref)) => {
                    self.view.handle = ModulePartHandle::new(module_ref, unsafe {
                        ModulePartRef::from_raw(ModulePartId::from_usize(0))
                    });
                }
                _ => {
                    return Err(Err(ErrorDiagnostic::new(
                        format!("`{}` is not a namespace", name),
                        self.source,
                    )));
                }
            }

            self.index += 1;
        }

        Ok(self.view)
    }
}
