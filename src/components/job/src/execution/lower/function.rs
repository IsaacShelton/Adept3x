use crate::{
    Continuation, Executable, ExecutionCtx, Executor, Suspend,
    execution::resolve::{ResolveFunctionBody, ResolveFunctionHead},
    module_graph::ModuleView,
    repr::{Compiler, FuncBody, FuncHead},
};
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct LowerFunction<'env> {
    view: ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    func: ByAddress<&'env ast::Func>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolving_head: Suspend<'env, &'env FuncHead<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_head: Option<&'env FuncHead<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolving_body: Suspend<'env, &'env FuncBody<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_body: Option<&'env FuncBody<'env>>,
}

impl<'env> LowerFunction<'env> {
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
            resolving_head: None,
            resolving_body: None,
        }
    }
}

impl<'env> Executable<'env> for LowerFunction<'env> {
    type Output = ();

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let _resolved_head = match self.resolved_head {
            Some(done) => done,
            None => {
                let Some(resolved_head) = executor.demand(self.resolving_head) else {
                    return suspend!(
                        self.resolving_head,
                        executor.request(ResolveFunctionHead::new(
                            self.view,
                            &self.compiler,
                            &self.func.head,
                        )),
                        ctx
                    );
                };
                self.resolved_head = Some(resolved_head);
                resolved_head
            }
        };

        let _resolved_body = match self.resolved_body {
            Some(done) => done,
            None => {
                let Some(resolved_body) = executor.demand(self.resolving_body) else {
                    return suspend!(
                        self.resolving_body,
                        executor.request(ResolveFunctionBody::new(
                            self.view,
                            &self.compiler,
                            &self.func,
                        )),
                        ctx
                    );
                };
                self.resolved_body = Some(resolved_body);
                resolved_body
            }
        };

        Ok(())
    }
}
