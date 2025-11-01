use crate::{
    Continuation, Executable, ExecutionCtx, Executor, Suspend,
    execution::{
        lower::{LowerFunctionBody, LowerFunctionHead},
        resolve::{ResolveFunctionBody, ResolveFunctionHead},
    },
    interpret::{Interpreter, syscall_handler::ComptimeSystemSyscallHandler},
    ir,
    module_graph::ModuleView,
    repr::{Evaluated, FuncBody, FuncHead},
};
use attributes::{Exposure, Privacy, SymbolOwnership, Tag};
use by_address::ByAddress;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ResolveEvaluation<'env> {
    comptime_view: &'env ModuleView<'env>,
    expr: ByAddress<&'env ast::Expr>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    ast_func: Option<&'env ast::Func>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_func_head: Suspend<'env, &'env FuncHead<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_func_body: Suspend<'env, &'env FuncBody<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    lowered_func_head: Suspend<'env, ir::FuncRef<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    lowered_func_body: Suspend<'env, ()>,
}

impl<'env> ResolveEvaluation<'env> {
    pub fn new(comptime_view: &'env ModuleView<'env>, expr: &'env ast::Expr) -> Self {
        Self {
            comptime_view,
            expr: ByAddress(expr),
            ast_func: None,
            resolved_func_head: None,
            resolved_func_body: None,
            lowered_func_head: None,
            lowered_func_body: None,
        }
    }
}

impl<'env> Executable<'env> for ResolveEvaluation<'env> {
    type Output = &'env Evaluated;

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        // 1) Create anonymous "comptime only" function to serve as the interpreter entry point
        let ast_func = self.ast_func.get_or_insert_with(|| {
            &*ctx.alloc(ast::Func {
                head: ast::FuncHead {
                    name: "".into(),
                    type_params: ast::TypeParams::default(),
                    givens: vec![],
                    params: ast::Params::default(),
                    return_type: ast::TypeKind::Void.at(self.expr.source),
                    source: self.expr.source,
                    abide_abi: false,
                    tag: Some(Tag::InterpreterEntryPoint),
                    privacy: Privacy::Private,
                    ownership: SymbolOwnership::Owned(Exposure::Hidden),
                },
                stmts: vec![
                    ast::StmtKind::ExitInterpreter(Box::new((*self.expr).clone()))
                        .at(self.expr.source),
                ],
                // NOTE: This should eventually be the same as the settings from the evaluation site
                settings: None,
            })
        });

        // 2) Resolve the function head and body
        let Some(resolved_func_head) = executor.demand(self.resolved_func_head) else {
            return suspend!(
                self.resolved_func_head,
                executor.request(ResolveFunctionHead::new(self.comptime_view, &ast_func.head)),
                ctx
            );
        };

        let Some(resolved_func_body) = executor.demand(self.resolved_func_body) else {
            return suspend!(
                self.resolved_func_body,
                executor.request(ResolveFunctionBody::new(
                    self.comptime_view,
                    ast_func,
                    resolved_func_head
                )),
                ctx
            );
        };

        // 3) Lower the function
        let Some(lowered_func_head) = executor.demand(self.lowered_func_head) else {
            return suspend!(
                self.lowered_func_head,
                executor.request(LowerFunctionHead::new(
                    self.comptime_view,
                    resolved_func_head
                )),
                ctx
            );
        };

        let Some(_lowered_func_body) = executor.demand(self.lowered_func_body) else {
            return suspend!(
                self.lowered_func_body,
                executor.request(LowerFunctionBody::new(
                    self.comptime_view,
                    lowered_func_head,
                    resolved_func_head,
                    resolved_func_body,
                )),
                ctx
            );
        };

        // 4) Obtain the intermediate representation for comptime so far
        let ir = self.comptime_view.graph(|graph| graph.ir);

        // 5) Interpret the function and raise any interpretation errors
        let mut interpreter =
            Interpreter::new(ComptimeSystemSyscallHandler::default(), ir, Some(1_000_000));

        let entry_point_result = interpreter
            .run(lowered_func_head)
            .map_err(|e| ErrorDiagnostic::new(format!("{}", e), self.expr.source))?;

        // The actual entry point result should be void
        entry_point_result.kind.unwrap_literal().unwrap_void();

        // 6) Examine the result value that was baked by the function
        let exit_value = interpreter.exit_value();

        // 7) Expect that the exit value is transferrable
        let Some(exit_value) = exit_value else {
            return Err(ErrorDiagnostic::new(
                "Compile-time evaluation must evaluate to transferable value",
                self.expr.source,
            )
            .into());
        };

        // 8) Translate the constant value into a literal value
        // and/or static data that can be used as a literal.
        Ok(ctx.alloc(Evaluated::new_unsigned(exit_value)))

        // TODO: We need to be able to support different types than just unsigned values, such as
        // booleans, etc.
        // Ok(ctx.alloc(Evaluated::new_boolean(true)))
    }
}
