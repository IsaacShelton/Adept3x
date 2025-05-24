use super::{Executable, FindTypeInEstimated};
use crate::{
    Continuation, ExecutionCtx, Executor, Suspend,
    repr::{DeclScope, FindTypeResult},
};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct FindType<'env> {
    name: &'env str,
    arity: usize,

    #[derivative(Debug = "ignore")]
    decl_scope: ByAddress<&'env DeclScope>,

    #[derivative(Debug = "ignore")]
    workspace: ByAddress<&'env AstWorkspace<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    estimation_pass: Suspend<'env, FindTypeResult>,
}

impl<'env> FindType<'env> {
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
                &self.decl_scope,
                self.name,
                self.arity
            )),
            ctx
        )
    }
}
