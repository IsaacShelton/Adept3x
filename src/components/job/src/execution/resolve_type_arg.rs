use super::{Executable, ResolveType};
use crate::{
    Continuation, ExecutionCtx, Executor, Suspend,
    repr::{DeclScope, Type, TypeArg},
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

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    inner_type: Suspend<'env, &'env Type<'env>>,
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
            inner_type: None,
        }
    }
}

impl<'env> Executable<'env> for ResolveTypeArg<'env> {
    type Output = &'env TypeArg<'env>;

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        match &**self.type_arg {
            ast::TypeArg::Type(inner) => {
                let Some(inner_type) = executor.demand(self.inner_type) else {
                    return suspend!(
                        self.inner_type,
                        executor.request(ResolveType::new(
                            &self.workspace,
                            inner,
                            &self.decl_scope
                        )),
                        ctx
                    );
                };

                Ok(ctx.alloc(TypeArg::Type(inner_type)))
            }
            ast::TypeArg::Expr(_) => {
                unimplemented!("non-type arguments to types are not supported yet")
            }
        }
    }
}
