use crate::{Continuation, Executable, ExecutionCtx, Executor, module_graph::ModuleView};
use ast::Namespace;
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ProcessNamespace<'env> {
    view: &'env ModuleView<'env>,
    namespace: Option<ByAddress<&'env Namespace>>,
}

impl<'env> ProcessNamespace<'env> {
    pub fn new(view: &'env ModuleView<'env>, namespace: &'env Namespace) -> Self {
        Self {
            view,
            namespace: Some(ByAddress(namespace)),
        }
    }
}

impl<'env> Executable<'env> for ProcessNamespace<'env> {
    type Output = ();

    fn execute(
        mut self,
        _executor: &Executor<'env>,
        _ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let Some(namespace) = self.namespace.take() else {
            return Ok(());
        };

        match &namespace.items {
            ast::NamespaceItemsSource::Items(_namespace_items) => {
                todo!("namespace items not supported yet")
            }
            ast::NamespaceItemsSource::Expr(_expr) => {
                todo!("namespace items from expr not supported")
            }
        }
    }
}
