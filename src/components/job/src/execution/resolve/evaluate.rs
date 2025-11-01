use crate::{
    Continuation, Executable, ExecutionCtx, Executor, Suspend,
    execution::{
        lower::{LowerFunctionBody, LowerFunctionHead},
        resolve::{ResolveFunctionBody, ResolveFunctionHead},
    },
    ir,
    module_graph::ModuleView,
    repr::{Evaluated, FuncBody, FuncHead},
};
use attributes::{Exposure, Privacy, SymbolOwnership, Tag};
use by_address::ByAddress;
use derivative::Derivative;

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

        self.comptime_view.graph(|graph| {
            let ir_func = &graph.ir.funcs[lowered_func_head];
            dbg!(ir_func);
        });

        dbg!(_lowered_func_body);

        // 4) Interpret the function.
        todo!("interpret func ref {:?}", lowered_func_head);

        // 5) Examine the result value that was baked by the function

        // 6) Raise error message if took too long

        // 7) Panic if not set, (or was set multiple times), as this
        // should never happen

        // 8) Translate the constant value into a literal value
        // and/or static data that can be used as a literal.

        Ok(ctx.alloc(Evaluated::new_boolean(true)))
    }
}
