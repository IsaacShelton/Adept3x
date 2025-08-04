use super::Executable;
use crate::{Continuation, ExecutionCtx, Executor, module_graph::ModuleView, repr::FindTypeResult};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct FindType<'env> {
    name: &'env str,
    arity: usize,

    #[derivative(Debug = "ignore")]
    view: ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    workspace: ByAddress<&'env AstWorkspace<'env>>,
}

impl<'env> FindType<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        view: ModuleView<'env>,
        name: &'env str,
        arity: usize,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            view,
            name,
            arity,
        }
    }
}

impl<'env> Executable<'env> for FindType<'env> {
    type Output = FindTypeResult;

    fn execute(
        self,
        _executor: &Executor<'env>,
        _ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        todo!("FindType::execute")
    }
}
