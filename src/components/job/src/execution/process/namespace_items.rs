use crate::{
    Continuation, Executable, ExecutionCtx, Executor, ProcessLinkset, ProcessPragma,
    ProcessStructure,
    execution::process::{ProcessFunction, ProcessNamespace, ProcessWhen},
    module_graph::ModuleView,
    repr::Compiler,
};
use ast::NamespaceItems;
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ProcessNamespaceItems<'env> {
    view: &'env ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    namespace_items: Option<ByAddress<&'env NamespaceItems>>,
}

impl<'env> ProcessNamespaceItems<'env> {
    pub fn new(
        view: &'env ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        namespace_items: &'env NamespaceItems,
    ) -> Self {
        Self {
            view,
            compiler: ByAddress(compiler),
            namespace_items: Some(ByAddress(namespace_items)),
        }
    }
}

impl<'env> Executable<'env> for ProcessNamespaceItems<'env> {
    type Output = ();

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let Some(namespace_items) = self.namespace_items.take() else {
            // All items resolved!
            return Ok(());
        };

        ctx.suspend_on(
            namespace_items
                .whens
                .iter()
                .map(|when| executor.request(ProcessWhen::new(self.view, &self.compiler, when))),
        );

        ctx.suspend_on(namespace_items.namespaces.iter().map(|namespace| {
            executor.request(ProcessNamespace::new(self.view, &self.compiler, namespace))
        }));

        ctx.suspend_on(
            namespace_items.pragmas.iter().map(|pragma| {
                executor.request(ProcessPragma::new(self.view, &self.compiler, pragma))
            }),
        );

        ctx.suspend_on(namespace_items.linksets.iter().map(|linkset| {
            executor.request(ProcessLinkset::new(self.view, &self.compiler, linkset))
        }));

        ctx.suspend_on(namespace_items.structs.iter().map(|structure| {
            executor.request(ProcessStructure::new(self.view, &self.compiler, structure))
        }));

        ctx.suspend_on(
            namespace_items.funcs.iter().map(|func| {
                executor.request(ProcessFunction::new(self.view, &self.compiler, func))
            }),
        );

        Err(Continuation::Suspend(self.into()))
    }
}
