use crate::{
    Continuation, Executable, ExecutionCtx, Executor, Suspend,
    execution::{
        build_ir::LowerFunctionHead,
        resolve::{ResolveFunctionBody, ResolveFunctionHead},
    },
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

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    lowering_head: Suspend<'env, ()>,
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
            resolving_head: None,
            resolving_body: None,
            lowering_head: None,
        }
    }
}

impl<'env> Executable<'env> for ResolveFunction<'env> {
    type Output = ();

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        // NOTE: We have some extra caching here, although this should probably be standardized
        let resolved_head = match self.resolved_head {
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

        if !self.view.graph.is_runtime() {
            // If not for runtime, we lazily process function bodies...
            return Ok(());
        }

        let Some(_lowered_head) = self.lowering_head else {
            return suspend!(
                self.lowering_head,
                executor.request(LowerFunctionHead::new(
                    self.view,
                    &self.compiler,
                    resolved_head
                )),
                ctx
            );
        };

        // NOTE: We have some extra caching here, although this should probably be standardized
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
