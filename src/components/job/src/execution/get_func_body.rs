use super::Executable;
use crate::{
    Continuation, ExecutionCtx, Executor, SuspendMany,
    repr::{DeclScope, FuncBody, Type},
};
use ast_workspace::{AstWorkspace, FuncRef};
use by_address::ByAddress;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct GetFuncBody<'env> {
    func_ref: FuncRef,

    #[derivative(Debug = "ignore")]
    workspace: ByAddress<&'env AstWorkspace<'env>>,

    #[derivative(Hash = "ignore")]
    decl_scope: ByAddress<&'env DeclScope>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    inner_types: SuspendMany<'env, &'env Type<'env>>,
}

impl<'env> GetFuncBody<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        func_ref: FuncRef,
        decl_scope: &'env DeclScope,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            func_ref,
            decl_scope: ByAddress(decl_scope),
            inner_types: None,
        }
    }
}

impl<'env> Executable<'env> for GetFuncBody<'env> {
    type Output = &'env FuncBody<'env>;

    #[allow(unused_variables)]
    #[allow(unreachable_code)]
    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let def = &self.workspace.symbols.all_funcs[self.func_ref];

        Err(ErrorDiagnostic::new(
            "Computing function bodies is not implemented yet!",
            def.head.source,
        )
        .into())

        // Ok(ctx.alloc(todo!("compute func body")))
    }
}
