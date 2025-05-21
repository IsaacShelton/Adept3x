use super::Executable;
use crate::{Continuation, ExecutionCtx, Executor, repr::DeclScope};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FindTypeInEstimated<'env> {
    workspace: ByAddress<&'env AstWorkspace<'env>>,
    decl_scope: ByAddress<&'env DeclScope>,
    name: &'env str,
    arity: usize,
}

impl<'env> FindTypeInEstimated<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        decl_scope: &'env DeclScope,
        name: &'env str,
        arity: usize,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            decl_scope: ByAddress(decl_scope),
            name,
            arity,
        }
    }
}

impl<'env> Executable<'env> for FindTypeInEstimated<'env> {
    type Output = Option<&'env asg::Type>;

    fn execute(
        self,
        _executor: &Executor<'env>,
        _ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let _workspace = self.workspace.0;
        let _decl_scope = self.decl_scope.0;

        Ok(None)
    }
}
