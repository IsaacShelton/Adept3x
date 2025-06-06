use super::{Executable, GetTypeHead};
use crate::{
    Continuation, ExecutionCtx, Executor, SuspendMany,
    repr::{DeclScope, TypeHead},
};
use ast_workspace::AstWorkspace;
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct EstimateTypeHeads<'env> {
    name: &'env str,

    #[derivative(Debug = "ignore")]
    workspace: ByAddress<&'env AstWorkspace<'env>>,

    #[derivative(Debug = "ignore")]
    decl_scope: ByAddress<&'env DeclScope>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    type_head_tasks: SuspendMany<'env, &'env TypeHead<'env>>,
}

impl<'env> EstimateTypeHeads<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        decl_scope: &'env DeclScope,
        name: &'env str,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            decl_scope: ByAddress(decl_scope),
            name,
            type_head_tasks: None,
        }
    }
}

impl<'env> Executable<'env> for EstimateTypeHeads<'env> {
    type Output = &'env [&'env TypeHead<'env>];

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let workspace = self.workspace.0;

        if let Some(type_heads) = executor.demand_many(&self.type_head_tasks) {
            return Ok(ctx.alloc_slice_fill_iter(type_heads));
        }

        let decl_set = self.decl_scope.get(&self.name);

        suspend_many!(
            self.type_head_tasks,
            executor.request_many(
                decl_set
                    .into_iter()
                    .flat_map(|decl_set| decl_set.type_decls())
                    .map(|type_decl_ref| GetTypeHead::new(workspace, type_decl_ref))
            ),
            ctx
        )
    }
}
