use crate::{
    Continuation, Executable, ExecutionCtx, Executor, ResolveFunction, ResolveNamespace,
    ResolveWhen, module_graph::ModuleView, repr::Compiler,
};
use ast::NamespaceItems;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ResolveNamespaceItems<'env> {
    view: ModuleView<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    compiler: &'env Compiler<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    namespace_items: Option<&'env NamespaceItems>,
}

impl<'env> ResolveNamespaceItems<'env> {
    pub fn new(
        view: ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        namespace_items: &'env NamespaceItems,
    ) -> Self {
        Self {
            view,
            compiler,
            namespace_items: Some(namespace_items),
        }
    }
}

impl<'env> Executable<'env> for ResolveNamespaceItems<'env> {
    type Output = ();

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let Some(namespace_items) = self.namespace_items.take() else {
            return Ok(());
        };

        ctx.suspend_on(
            namespace_items
                .whens
                .iter()
                .map(|when| executor.spawn_raw(ResolveWhen::new(self.view, self.compiler, when))),
        );

        ctx.suspend_on(namespace_items.namespaces.iter().map(|namespace| {
            executor.spawn_raw(ResolveNamespace::new(self.view, self.compiler, namespace))
        }));

        ctx.suspend_on(
            namespace_items.funcs.iter().map(|func| {
                executor.spawn_raw(ResolveFunction::new(self.view, self.compiler, &func))
            }),
        );

        Err(Continuation::Suspend(self.into()))
    }
}
