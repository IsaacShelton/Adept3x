use crate::{
    Continuation, Executable, ExecutionCtx, Executor,
    execution::main::LoadFile,
    module_graph::{ModuleView, Upserted},
    repr::Compiler,
};
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct EvaluateComptime<'env> {
    view: ModuleView<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    compiler: &'env Compiler<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    expr: ast::Expr,
}

impl<'env> EvaluateComptime<'env> {
    pub fn new(view: ModuleView<'env>, compiler: &'env Compiler<'env>, expr: ast::Expr) -> Self {
        Self {
            view,
            compiler,
            expr,
        }
    }
}

impl<'env> Executable<'env> for EvaluateComptime<'env> {
    type Output = bool;

    fn execute(
        self,
        executor: &Executor<'env>,
        _ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
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
            let _ = executor.spawn(LoadFile::new(
                self.compiler,
                created.canonical_module_filename,
                created,
                None,
            ));
        }

        let comptime_module = comptime_module.out_of();
        comptime_module.upsert_part(self.view.canonical_filename);

        //todo!("comptime evaluate expr {:?}", comptime_graph);
        Ok(true)
    }
}
