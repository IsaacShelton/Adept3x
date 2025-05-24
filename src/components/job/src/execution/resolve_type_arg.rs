use super::Executable;
use crate::{
    Continuation, ExecutionCtx, Executor,
    repr::{DeclScope, TypeArg},
};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ResolveTypeArg<'env> {
    type_arg: ByAddress<&'env ast::TypeArg>,

    #[derivative(Debug = "ignore")]
    workspace: ByAddress<&'env AstWorkspace<'env>>,

    #[derivative(Debug = "ignore")]
    decl_scope: ByAddress<&'env DeclScope>,
}

impl<'env> ResolveTypeArg<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        type_arg: &'env ast::TypeArg,
        decl_scope: &'env DeclScope,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            type_arg: ByAddress(type_arg),
            decl_scope: ByAddress(decl_scope),
        }
    }
}

impl<'env> Executable<'env> for ResolveTypeArg<'env> {
    type Output = &'env TypeArg<'env>;

    fn execute(
        self,
        _executor: &Executor<'env>,
        _ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        todo!("resolve type arg")
    }
}
