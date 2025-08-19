use crate::{
    Continuation, EvaluateComptime, Executable, ExecutionCtx, Executor, ResolveNamespaceItems,
    Suspend, module_graph::ModuleView, repr::Compiler,
};
use ast::{NamespaceItems, When};
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ResolveWhen<'env> {
    view: ModuleView<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    compiler: &'env Compiler<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    conditions_stack: Vec<(&'env ast::Expr, &'env NamespaceItems)>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    otherwise: Option<&'env NamespaceItems>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    condition: Suspend<'env, bool>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    then: Option<&'env NamespaceItems>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    did_spawn: bool,
}

impl<'env> ResolveWhen<'env> {
    pub fn new(view: ModuleView<'env>, compiler: &'env Compiler<'env>, when: &'env When) -> Self {
        Self {
            view,
            compiler,
            conditions_stack: when.conditions.iter().rev().map(|(a, b)| (a, b)).collect(),
            otherwise: when.otherwise.as_ref(),
            condition: None,
            then: None,
            did_spawn: false,
        }
    }
}

impl<'env> Executable<'env> for ResolveWhen<'env> {
    type Output = ();

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        // Was waiting on items to complete
        if self.did_spawn {
            return Ok(());
        };

        // Was waiting on evaluation
        if let Some(yes) = executor.demand(self.condition) {
            self.condition = None;
            if yes {
                self.did_spawn = true;
                ctx.suspend_on(std::iter::once(executor.spawn_raw(
                    ResolveNamespaceItems::new(self.view, self.compiler, self.then.unwrap()),
                )));
                return Err(Continuation::Suspend(self.into()));
            }
        }

        // Suspend on next condition if there is one
        if let Some((condition, items)) = self.conditions_stack.pop() {
            self.then = Some(items);
            return suspend!(
                self.condition,
                executor.spawn(EvaluateComptime::new(self.view, self.compiler, condition)),
                ctx
            );
        }

        // Suspend on "else" items if present and no condition was met
        if let Some(otherwise) = self.otherwise.take() {
            self.did_spawn = true;
            ctx.suspend_on(std::iter::once(executor.spawn_raw(
                ResolveNamespaceItems::new(self.view, self.compiler, otherwise),
            )));
            return Err(Continuation::Suspend(self.into()));
        }

        // Nothing to do, all conditions evaluated to false and there is no "else" branch.
        Ok(())
    }
}
