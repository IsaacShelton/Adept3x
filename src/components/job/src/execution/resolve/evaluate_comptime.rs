use crate::{
    Continuation, Executable, ExecutionCtx, Executor, ProcessFile, RequireFileHeader, Suspend,
    execution::resolve::ResolveEvaluation,
    module_graph::{ModuleView, Upserted},
    repr::Evaluated,
};
use by_address::ByAddress;
use derivative::Derivative;
use std_ext::SmallVec2;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct EvaluateComptime<'env> {
    view: &'env ModuleView<'env>,

    expr: ByAddress<&'env ast::Expr>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    evaluated: Suspend<'env, &'env Evaluated>,
}

impl<'env> EvaluateComptime<'env> {
    pub fn new(view: &'env ModuleView<'env>, expr: &'env ast::Expr) -> Self {
        Self {
            view,
            expr: ByAddress(expr),
            evaluated: None,
        }
    }
}

impl<'env> Executable<'env> for EvaluateComptime<'env> {
    type Output = &'env Evaluated;

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        if let Some(evaluated) = executor.demand(self.evaluated) {
            return Ok(evaluated);
        }

        let comptime_graph = self
            .view
            .graph
            .default_comptime()
            .unwrap_or(self.view.graph);

        let comptime_meta = self.view.web.graph(comptime_graph, |graph| graph.meta());

        let comptime_module = self.view.web.upsert_module_with_initial_part(
            comptime_graph,
            comptime_meta,
            self.view.canonical_module_filename,
        );

        let mut wait_for = SmallVec2::new();

        if let Upserted::Created(created) = comptime_module {
            wait_for.push(
                executor
                    .request(ProcessFile::new(
                        self.view.compiler(),
                        created.canonical_module_filename,
                        RequireFileHeader::Ignore,
                        ctx.alloc(created),
                        None,
                    ))
                    .raw_task_ref(),
            );
        }

        let comptime_module = comptime_module.out_of();
        let comptime_part_view = comptime_module.upsert_part(self.view.canonical_filename);

        if let Upserted::Created(created) = comptime_part_view {
            let processing_file = executor
                .request(ProcessFile::new(
                    self.view.compiler(),
                    created.canonical_filename,
                    RequireFileHeader::Ignore,
                    ctx.alloc(created),
                    None,
                ))
                .raw_task_ref();

            if !wait_for.contains(&processing_file) {
                wait_for.push(processing_file);
            }
        }

        let comptime_part_view = ctx.alloc(comptime_part_view.out_of());

        // We wait on the files /comptime/ processing (which includes function heads, etc.)
        // so we get consistent error messages. It's possible that symbols are invalidated after
        // we've evaluated the expression that wouldn't have the time to raise errors otherwise.
        // TODO: This waiting should actually happen at a higher level though, since it's
        // possible that a function head or similar can depend on a compile-time evaluation
        // within the same scope and comptime-ness graph.
        if !wait_for.is_empty() {
            ctx.suspend_on(wait_for.drain(..));
        }

        return suspend!(
            self.evaluated,
            executor.request(ResolveEvaluation::new(comptime_part_view, &self.expr)),
            ctx
        );
    }
}
