use crate::{
    Continuation, Executable, ExecutionCtx, Executor, Suspend,
    execution::{
        lower::{LowerFunctionBody, LowerFunctionHead},
        resolve::{ResolveFunctionBody, ResolveFunctionHead},
    },
    ir,
    module_graph::ModuleView,
    repr::{FuncBody, FuncHead},
};
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ProcessFunction<'env> {
    view: &'env ModuleView<'env>,

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
    lowering_head: Suspend<'env, ir::FuncRef<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    lowering_body: Suspend<'env, ()>,
}

impl<'env> ProcessFunction<'env> {
    pub fn new(view: &'env ModuleView<'env>, func: &'env ast::Func) -> Self {
        Self {
            view,
            func: ByAddress(func),
            resolved_head: None,
            resolved_body: None,
            resolving_head: None,
            resolving_body: None,
            lowering_head: None,
            lowering_body: None,
        }
    }
}

impl<'env> Executable<'env> for ProcessFunction<'env> {
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
                        executor.request(ResolveFunctionHead::new(self.view, &self.func)),
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

        // Don't process bodies for foreign functions
        if self.func.head.ownership.is_reference() {
            return Ok(());
        }

        // NOTE: We have some extra caching here, although this should probably be standardized
        let resolved_body = match self.resolved_body {
            Some(done) => done,
            None => {
                let Some(resolved_body) = executor.demand(self.resolving_body) else {
                    return suspend!(
                        self.resolving_body,
                        executor.request(ResolveFunctionBody::new(self.view, resolved_head)),
                        ctx
                    );
                };
                self.resolved_body = Some(resolved_body);
                resolved_body
            }
        };

        // Don't lower generic functions unless specifically requested
        if self.func.head.is_generic() {
            return Ok(());
        }

        let Some(lowered_head) = executor.demand(self.lowering_head) else {
            return suspend!(
                self.lowering_head,
                executor.request(LowerFunctionHead::new(resolved_head)),
                ctx
            );
        };

        let Some(_lowered_body) = executor.demand(self.lowering_body) else {
            return suspend!(
                self.lowering_body,
                executor.request(LowerFunctionBody::new(
                    lowered_head,
                    resolved_head,
                    resolved_body
                )),
                ctx
            );
        };

        Ok(())
    }
}
