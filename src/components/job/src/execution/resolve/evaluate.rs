use crate::{
    CfgBuilder, Continuation, Executable, ExecutionCtx, Executor, IsValue, flatten_expr,
    module_graph::ModuleView,
    repr::{Evaluated, TypeDisplayerDisambiguation},
};
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ResolveEvaluation<'env> {
    view: &'env ModuleView<'env>,
    expr: ByAddress<&'env ast::Expr>,
}

impl<'env> ResolveEvaluation<'env> {
    pub fn new(view: &'env ModuleView<'env>, expr: &'env ast::Expr) -> Self {
        Self {
            view,
            expr: ByAddress(expr),
        }
    }
}

impl<'env> Executable<'env> for ResolveEvaluation<'env> {
    type Output = &'env Evaluated;

    fn execute(
        self,
        _executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let (mut comptime_builder, mut comptime_cursor) = CfgBuilder::new();

        let cfg_value = flatten_expr(
            ctx,
            &mut comptime_builder,
            &mut comptime_cursor,
            self.expr.0,
            IsValue::RequireValue,
        );

        dbg!(self.expr);
        eprintln!(
            "{}",
            comptime_builder.display(&self.view, &TypeDisplayerDisambiguation::empty())
        );

        // 0) Get the corresponding view in the (relatively) comptime world

        // 1) Create anonymous "comptime only" function
        // to serve as the interpreter entry point (comptime view)

        // 2) Create a value to represent the schema within
        // the interpreter

        // 3) Add in section at end of builder to
        // call a syscall/intrinsic-instr with the resulting value and schema.

        // 4) Finish the function (comptime view).

        // 5) Lower the function (for comptime view).

        // 6) Interpret the function.

        // 7) Examine the result value that was baked by the function

        // 8) Raise error message if took too long

        // 9) Panic if not set, (or was set multiple times), as this
        // should never happen

        // 10) Translate the constant value into a literal value
        // and/or static data that can be used as a literal.

        Ok(ctx.alloc(Evaluated::Bool(true)))
    }
}
