use crate::{
    Continuation, Executable, ExecutionCtx, Executor, InstrKind, Pending, Suspend,
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
use itertools::Itertools;
use std::collections::HashMap;

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

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    transitive_func_state: TransitiveFuncState<'env>,
}

/// State machine for completing transitive function dependencies
#[derive(Clone)]
enum TransitiveFuncState<'env> {
    Initialize,
    Step {
        scan_next: Vec<&'env FuncBody<'env>>,
        resolved_bodies: HashMap<ByAddress<&'env FuncHead<'env>>, &'env FuncBody<'env>>,
        requests: HashMap<ByAddress<&'env FuncHead<'env>>, Pending<'env, &'env FuncBody<'env>>>,
    },
    LowerBodies(
        Vec<(
            &'env FuncHead<'env>,
            &'env FuncBody<'env>,
            Pending<'env, ir::FuncRef<'env>>,
        )>,
    ),
    Finished,
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
            transitive_func_state: TransitiveFuncState::Initialize,
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
                executor.request(ResolveFunctionHead::new(self.comptime_view, &ast_func)),
                ctx
            );
        };

        let Some(resolved_func_body) = executor.demand(self.resolved_func_body) else {
            return suspend!(
                self.resolved_func_body,
                executor.request(ResolveFunctionBody::new(
                    self.comptime_view,
                    resolved_func_head
                )),
                ctx
            );
        };

        // 3) Resolve transitive function dependencies
        if let TransitiveFuncState::Initialize = &self.transitive_func_state {
            self.transitive_func_state = TransitiveFuncState::Step {
                scan_next: vec![resolved_func_body],
                resolved_bodies: HashMap::new(),
                requests: HashMap::new(),
            };
        }

        match &mut self.transitive_func_state {
            TransitiveFuncState::Initialize => unreachable!(),
            TransitiveFuncState::Step {
                scan_next,
                resolved_bodies,
                requests,
            } => {
                // Receive completed resolved function bodies
                {
                    let truth = executor.truth.read().unwrap();
                    for (head, pending) in std::mem::take(requests) {
                        let body = truth.demand(pending);
                        resolved_bodies.insert(head, body);
                        scan_next.push(body);
                    }
                }

                // Search newly resolved function bodies
                for body in scan_next.drain(..) {
                    for instr in body.cfg.iter_instrs_unordered() {
                        // For any function calls made
                        if let InstrKind::Call(_, call_target) = &instr.kind {
                            let call_target = call_target.as_ref().unwrap();
                            let key = ByAddress(call_target.callee);

                            // If we didn't already resolve the callee and aren't already planning
                            // to request the callee to be resolved
                            if !resolved_bodies.contains_key(&key) && !requests.contains_key(&key) {
                                // Then add the callee to the set of functions to resolve in the next
                                // suspend.
                                requests.insert(
                                    key,
                                    executor.request(ResolveFunctionBody::new(key.view, key.0)),
                                );
                            }
                        }
                    }
                }

                // Suspend and continue stepping if any requests need to be made
                if !requests.is_empty() {
                    ctx.suspend_on(requests.iter().map(|(_, pending)| pending));
                    return Err(Continuation::suspend(self));
                }

                // Otherwise, all function CFG bodies have been resolved,
                // and now we have to lower all of the function heads used...
                let mut pending_heads = Vec::new();
                for (head, body) in resolved_bodies {
                    let request = executor.request(LowerFunctionHead::new(head));
                    pending_heads.push((**head, *body, request));
                }

                // Suspend on all function heads being lowered that we need
                ctx.suspend_on(pending_heads.iter().map(|(_, _, pending)| pending));
                self.transitive_func_state = TransitiveFuncState::LowerBodies(pending_heads);
                return Err(Continuation::suspend(self));
            }
            TransitiveFuncState::LowerBodies(pending_heads) => {
                // Extract lowered function heads that we waited on
                let pending = {
                    let truth = executor.truth.read().unwrap();
                    pending_heads
                        .into_iter()
                        .map(|(head, body, request)| {
                            let ir_func_ref = truth.demand(*request);
                            LowerFunctionBody::new(*ir_func_ref, head, body)
                        })
                        .collect_vec()
                };

                // Suspend on all function bodies being lowered that we need
                ctx.suspend_on(executor.request_many(pending.into_iter()));
                self.transitive_func_state = TransitiveFuncState::Finished;
                return Err(Continuation::suspend(self));
            }
            TransitiveFuncState::Finished => (),
        }

        // 4) Lower the entry point function head
        let Some(lowered_func_head) = executor.demand(self.lowered_func_head) else {
            return suspend!(
                self.lowered_func_head,
                executor.request(LowerFunctionHead::new(resolved_func_head)),
                ctx
            );
        };

        // 5) Lower the entry point function body
        let Some(_lowered_func_body) = executor.demand(self.lowered_func_body) else {
            return suspend!(
                self.lowered_func_body,
                executor.request(LowerFunctionBody::new(
                    lowered_func_head,
                    resolved_func_head,
                    resolved_func_body,
                )),
                ctx
            );
        };

        // 6) Obtain the intermediate representation for comptime so far
        let ir = self.comptime_view.graph(|graph| graph.ir);

        // 7) Interpret the function and raise any interpretation errors
        let mut interpreter =
            Interpreter::new(ComptimeSystemSyscallHandler::default(), ir, Some(1_000_000));

        let entry_point_result = interpreter
            .run(lowered_func_head)
            .map_err(|e| ErrorDiagnostic::new(format!("{}", e), self.expr.source))?;

        // The actual entry point result should be void
        entry_point_result.kind.unwrap_literal().unwrap_void();

        // 8) Examine the result value that was baked by the function
        let exit_value = interpreter.exit_value();

        // 9) Expect that the exit value is transferrable
        let Some(exit_value) = exit_value else {
            return Err(ErrorDiagnostic::new(
                "Compile-time evaluation must evaluate to transferable value",
                self.expr.source,
            )
            .into());
        };

        // 10) Translate the constant value into a literal value
        // and/or static data that can be used as a literal.
        Ok(ctx.alloc(Evaluated::new_unsigned(exit_value)))

        // TODO: We need to be able to support different types than just unsigned values, such as
        // booleans, etc.
        // Ok(ctx.alloc(Evaluated::new_boolean(true)))
    }
}
