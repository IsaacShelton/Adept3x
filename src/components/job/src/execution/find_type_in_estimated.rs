use super::Executable;
use crate::{
    Continuation, EstimateDeclScope, ExecutionCtx, Executor, Suspend,
    execution::find_type_in_decl_set::FindTypeInDeclSet,
    repr::{DeclScope, DeclScopeOrigin, FindTypeResult},
};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;
use derive_more::Debug;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct FindTypeInEstimated<'env> {
    #[debug(skip)]
    workspace: ByAddress<&'env AstWorkspace<'env>>,
    starting_decl_scope: DeclScopeOrigin,
    name: &'env str,
    arity: usize,
    #[debug(skip)]
    estimated_decl_scope: Suspend<'env, &'env DeclScope>,
    #[debug(skip)]
    find_in_decl_set: Suspend<'env, FindTypeResult>,
    #[debug(skip)]
    exhausted: bool,
}

impl<'env> FindTypeInEstimated<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        starting_decl_scope: DeclScopeOrigin,
        name: &'env str,
        arity: usize,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            starting_decl_scope,
            name,
            arity,
            estimated_decl_scope: None,
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
        let workspace = self.workspace.0;

        if let Some(find) = executor.demand(self.find_in_decl_set) {
            return Ok(find);
        }

        if let Some(decl_scope) = executor.demand(self.estimated_decl_scope) {
            let Some(decl_set) = decl_scope.get(self.name) else {
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

        suspend!(
            self.estimated_decl_scope,
            executor.request(EstimateDeclScope::new(workspace, self.starting_decl_scope)),
            ctx
        )
    }
}
