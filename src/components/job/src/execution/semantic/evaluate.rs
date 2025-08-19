use crate::{
    Continuation, Executable, ExecutionCtx, Executor,
    module_graph::ModuleView,
    repr::{Compiler, Evaluated},
};
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ResolveEvaluation<'env> {
    view: ModuleView<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    compiler: &'env Compiler<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    expr: &'env ast::Expr,
}

impl<'env> ResolveEvaluation<'env> {
    pub fn new(
        view: ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        expr: &'env ast::Expr,
    ) -> Self {
        Self {
            view,
            compiler,
            expr,
        }
    }
}

impl<'env> Executable<'env> for ResolveEvaluation<'env> {
    type Output = &'env Evaluated;

    fn execute(
        self,
        _executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        Ok(ctx.alloc(Evaluated::Bool(true)))
    }
}
