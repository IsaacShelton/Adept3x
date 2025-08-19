use crate::{
    Continuation, Executable, ExecutionCtx, Executor, ResolveFunctionHead, Suspend,
    module_graph::ModuleView, repr::Compiler,
};
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ResolveFunction<'env> {
    view: ModuleView<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    compiler: &'env Compiler<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    func: &'env ast::Func,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_head: Suspend<'env, ()>,
}

impl<'env> ResolveFunction<'env> {
    pub fn new(
        view: ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        func: &'env ast::Func,
    ) -> Self {
        Self {
            view,
            compiler,
            func,
            resolved_head: None,
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
        let Some(_resolved_head) = executor.demand(self.resolved_head) else {
            return suspend!(
                self.resolved_head,
                executor.request(ResolveFunctionHead::new(
                    self.view,
                    self.compiler,
                    &self.func.head,
                )),
                ctx
            );
        };

        todo!("resolve func")
    }
}
