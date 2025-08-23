use crate::{
    Continuation, Executable, ExecutionCtx, Executor, ProcessFile, Suspend,
    execution::resolve::ResolveEvaluation,
    module_graph::{ModuleView, Upserted},
    repr::{Compiler, Evaluated},
};
use by_address::ByAddress;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct EvaluateComptime<'env> {
    view: ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    expr: ByAddress<&'env ast::Expr>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    evaluated: Suspend<'env, &'env Evaluated>,
}

impl<'env> EvaluateComptime<'env> {
    pub fn new(
        view: ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        expr: &'env ast::Expr,
    ) -> Self {
        Self {
            view,
            compiler: ByAddress(compiler),
            expr: ByAddress(expr),
            evaluated: None,
        }
    }
}

impl<'env> Executable<'env> for EvaluateComptime<'env> {
    type Output = bool;

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        if let Some(evaluated) = executor.demand(self.evaluated) {
            match evaluated {
                Evaluated::Bool(whether) => return Ok(*whether),
                _ => {
                    return Err(
                        ErrorDiagnostic::plain("Expected bool from comptime evaluation").into(),
                    );
                }
            }
        }

        let comptime_graph = self
            .view
            .graph
            .default_comptime()
            .unwrap_or(self.view.graph);

        let comptime_module = self
            .view
            .web
            .upsert_module_with_initial_part(comptime_graph, self.view.canonical_module_filename);

        if let Upserted::Created(created) = comptime_module {
            let _ = executor.spawn_raw(ProcessFile::new(
                &self.compiler,
                created.canonical_module_filename,
                created,
                None,
            ));
        }

        let comptime_module = comptime_module.out_of();
        let comptime_part = comptime_module.upsert_part(self.view.canonical_filename);

        if let Upserted::Created(created) = comptime_part {
            let _ = executor.spawn_raw(ProcessFile::new(
                &self.compiler,
                created.canonical_filename,
                created,
                None,
            ));
        }
        let comptime_part = comptime_part.out_of();

        return suspend!(
            self.evaluated,
            executor.request(ResolveEvaluation::new(
                comptime_part,
                &self.compiler,
                &self.expr
            )),
            ctx
        );
    }
}
