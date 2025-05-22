use super::{Executable, FindTypeInEstimated};
use crate::{
    Continuation, ExecutionCtx, Executor, Suspend,
    repr::{DeclScopeOrigin, FindTypeResult},
};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;
use derive_more::Debug;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FindType<'env> {
    #[debug(skip)]
    workspace: ByAddress<&'env AstWorkspace<'env>>,
    decl_scope_origin: DeclScopeOrigin,
    #[debug(skip)]
    name: &'env str,
    arity: usize,
    #[debug(skip)]
    estimation_pass: Suspend<'env, FindTypeResult>,
}

impl<'env> FindType<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        decl_scope_origin: DeclScopeOrigin,
        name: &'env str,
        arity: usize,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            decl_scope_origin,
            name,
            arity,
            estimation_pass: None,
        }
    }
}

impl<'env> Executable<'env> for FindType<'env> {
    type Output = FindTypeResult;

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        if let Some(estimation_pass) = executor.demand(self.estimation_pass) {
            return Ok(estimation_pass);
        }

        suspend!(
            self.estimation_pass,
            executor.request(FindTypeInEstimated::new(
                &self.workspace,
                self.decl_scope_origin,
                self.name,
                self.arity
            )),
            ctx
        )
    }
}
