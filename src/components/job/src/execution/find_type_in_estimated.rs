use super::Executable;
use crate::{
    Continuation, ExecutionCtx, Executor, Suspend,
    execution::find_type_in_decl_set::FindTypeInDeclSet,
    repr::{DeclScope, FindTypeResult},
};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct FindTypeInEstimated<'env> {
    name: &'env str,
    arity: usize,

    #[derivative(Debug = "ignore")]
    workspace: ByAddress<&'env AstWorkspace<'env>>,

    #[derivative(Debug = "ignore")]
    estimated_decl_scope: ByAddress<&'env DeclScope>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    find_in_decl_set: Suspend<'env, FindTypeResult>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    exhausted: bool,
}

impl<'env> FindTypeInEstimated<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        estimated_decl_scope: &'env DeclScope,
        name: &'env str,
        arity: usize,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            estimated_decl_scope: ByAddress(estimated_decl_scope),
            name,
            arity,
            find_in_decl_set: None,
            exhausted: false,
        }
    }
}

impl<'env> Executable<'env> for FindTypeInEstimated<'env> {
    type Output = FindTypeResult;

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        if let Some(find) = executor.demand(self.find_in_decl_set) {
            return Ok(find);
        }

        let Some(decl_set) = self.estimated_decl_scope.get(self.name) else {
            return Ok(Ok(None));
        };

        return suspend!(
            self.find_in_decl_set,
            executor.request(FindTypeInDeclSet::new(
                &self.workspace,
                decl_set,
                self.arity
            )),
            ctx
        );
    }
}
