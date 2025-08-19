use crate::{
    Continuation, Executable, ExecutionCtx, Executor, Suspend,
    execution::semantic::{ResolveFunctionBody, ResolveFunctionHead},
    module_graph::ModuleView,
    repr::{Compiler, FuncBody, FuncHead},
};
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ResolveFunction<'env> {
    view: ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    func: ByAddress<&'env ast::Func>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_head: Suspend<'env, &'env FuncHead<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_body: Suspend<'env, &'env FuncBody<'env>>,
}

impl<'env> ResolveFunction<'env> {
    pub fn new(
        view: ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        func: &'env ast::Func,
    ) -> Self {
        Self {
            view,
            compiler: ByAddress(compiler),
            func: ByAddress(func),
            resolved_head: None,
            resolved_body: None,
        }
    }
}

impl<'env> Executable<'env> for ResolveFunction<'env> {
    type Output = ();

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let Some(resolved_head) = executor.demand(self.resolved_head) else {
            return suspend!(
                self.resolved_head,
                executor.request(ResolveFunctionHead::new(
                    self.view,
                    &self.compiler,
                    &self.func.head,
                )),
                ctx
            );
        };

        let Some(resolved_body) = executor.demand(self.resolved_body) else {
            return suspend!(
                self.resolved_body,
                executor.request(ResolveFunctionBody::new(
                    self.view,
                    &self.compiler,
                    &self.func,
                )),
                ctx
            );
        };

        todo!(
            "resolve func, has head - {:?} {:?}",
            resolved_head,
            resolved_body
        )
    }
}
