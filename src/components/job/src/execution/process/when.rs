use crate::{
    Continuation, Executable, ExecutionCtx, Executor, ProcessNamespaceItems, Suspend,
    execution::resolve::EvaluateComptime, module_graph::ModuleView, repr::Compiler,
};
use ast::When;
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ProcessWhen<'env> {
    view: &'env ModuleView<'env>,

    when: ByAddress<&'env When>,

    // The next condition/otherwise index to check, or None if
    // waiting on a chosen conditional's "then" branch.
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    next_condition_index: Option<usize>,

    // The current comptime evaluation being suspended on
    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    condition: Suspend<'env, bool>,
}

impl<'env> ProcessWhen<'env> {
    pub fn new(view: &'env ModuleView<'env>, when: &'env When) -> Self {
        Self {
            view,
            when: ByAddress(when),
            next_condition_index: Some(0),
            condition: None,
        }
    }
}

impl<'env> Executable<'env> for ProcessWhen<'env> {
    type Output = ();

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        // Was waiting on items to complete
        let Some(next_condition_index) = &mut self.next_condition_index else {
            return Ok(());
        };

        // Was waiting on evaluation
        if let Some(yes) = executor.demand(self.condition) {
            self.condition = None;
            if yes {
                let then = self
                    .when
                    .conditions
                    .get(*next_condition_index)
                    .map(|(_, then)| then)
                    .unwrap_or_else(|| self.when.otherwise.as_ref().unwrap());

                self.next_condition_index = None;
                ctx.suspend_on(std::iter::once(
                    executor.spawn_raw(ProcessNamespaceItems::new(self.view, then)),
                ));
                return Err(Continuation::Suspend(self.into()));
            } else {
                *next_condition_index += 1;
            }
        }

        // Suspend on next condition if there is one
        if *next_condition_index < self.when.conditions.len() {
            let (condition, _) = &self.when.conditions[*next_condition_index];

            return suspend!(
                self.condition,
                executor.spawn(EvaluateComptime::new(self.view, condition)),
                ctx
            );
        }

        // Suspend on "else" items if present and no condition was met
        if let Some(otherwise) = &self.when.otherwise {
            self.next_condition_index = None;
            ctx.suspend_on(std::iter::once(
                executor.spawn_raw(ProcessNamespaceItems::new(self.view, otherwise)),
            ));
            return Err(Continuation::Suspend(self.into()));
        }

        // Nothing to do, all conditions evaluated to false and there is no "else" branch.
        Ok(())
    }
}
