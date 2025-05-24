use super::Executable;
use crate::{
    Continuation, ExecutionCtx, Executor, GetTypeHead, SuspendManyAssoc,
    repr::{AmbiguousType, DeclSet, FindTypeResult, TypeHead},
};
use ast_workspace::{AstWorkspace, TypeDeclRef};
use by_address::ByAddress;
use derivative::Derivative;
use itertools::Itertools;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct FindTypeInDeclSet<'env> {
    arity: usize,

    #[derivative(Debug = "ignore")]
    decl_set: ByAddress<&'env DeclSet>,

    #[derivative(Debug = "ignore")]
    workspace: ByAddress<&'env AstWorkspace<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    type_heads: SuspendManyAssoc<'env, TypeDeclRef, &'env TypeHead<'env>>,
}

impl<'env> FindTypeInDeclSet<'env> {
    pub fn new(workspace: &'env AstWorkspace<'env>, decl_set: &'env DeclSet, arity: usize) -> Self {
        Self {
            workspace: ByAddress(workspace),
            decl_set: ByAddress(decl_set),
            arity,
            type_heads: None,
        }
    }
}

impl<'env> Executable<'env> for FindTypeInDeclSet<'env> {
    type Output = FindTypeResult;

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let workspace = self.workspace.0;

        if let Some(type_heads) = executor.demand_many_assoc(&self.type_heads) {
            let type_heads = type_heads
                .into_iter()
                .filter(|(_, type_head)| type_head.arity == self.arity);

            return Ok(type_heads
                .at_most_one()
                .map(|one| one.map(|(type_decl_ref, _type_head)| type_decl_ref))
                .map_err(|_| AmbiguousType));
        }

        suspend_many_assoc!(
            self.type_heads,
            executor.request_many_assoc(
                self.decl_set.type_decls().map(|type_decl_ref| (
                    type_decl_ref,
                    GetTypeHead::new(workspace, type_decl_ref)
                ))
            ),
            ctx
        )
    }
}
