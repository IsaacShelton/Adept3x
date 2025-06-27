mod basic_bin_op;
mod dominators;

use super::Executable;
use crate::{
    BuiltinTypes, Continuation, ExecutionCtx, Executor, ResolveType, Resolved, Suspend,
    SuspendMany, Value,
    cfg::{
        NodeId, NodeKind, NodeRef, SequentialNodeKind, UntypedCfg, flatten_func_ignore_const_evals,
    },
    conform::conform_to_default,
    repr::{DeclScope, FuncBody, Type, TypeKind},
    unify::unify_types,
};
use arena::ArenaMap;
use ast_workspace::{AstWorkspace, FuncRef};
use basic_bin_op::resolve_basic_binary_operation_expr_on_literals;
use by_address::ByAddress;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;
use dominators::compute_idom_tree;
use itertools::Itertools;
use primitives::CInteger;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct GetFuncBody<'env> {
    func_ref: FuncRef,

    #[derivative(Debug = "ignore")]
    workspace: ByAddress<&'env AstWorkspace<'env>>,

    #[derivative(Hash = "ignore")]
    decl_scope: ByAddress<&'env DeclScope>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    inner_types: SuspendMany<'env, &'env Type<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    types: ArenaMap<NodeId, Resolved<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    num_processed: usize,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    dominator_type: Suspend<'env, &'env Type<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    builtin_types: &'env BuiltinTypes<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    cfg: Option<&'env UntypedCfg>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    dominators_and_post_order: Option<&'env (ArenaMap<NodeId, NodeRef>, Vec<NodeRef>)>,
}

impl<'env> GetFuncBody<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        func_ref: FuncRef,
        decl_scope: &'env DeclScope,
        builtin_types: &'env BuiltinTypes<'env>,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            func_ref,
            decl_scope: ByAddress(decl_scope),
            inner_types: None,
            types: ArenaMap::new(),
            num_processed: 0,
            dominator_type: None,
            builtin_types,
            cfg: None,
            dominators_and_post_order: None,
        }
    }

    fn get_typed(&self, node_ref: NodeRef) -> &Resolved<'env> {
        self.types.get(node_ref.into_raw()).unwrap()
    }
}

/*
- For both implementation simplicity and mental processing simplicity, we will forbid gotos, break, and continue within expression aliases
    - Macros can be used if non-trivial control flow modifications are necessary, e.g. @await or @yield or @break(2) or @try etc.

#### Function Body Resolution Order
 - Flatten AST to CFG nodes and GOTO relocations
 - Resolve macros to CFG flows
 - Resolve GOTO relocations
 - Determine post order traversal
 - Determine immediate dominators tree
 - Resolve variable names to corresponding nodes
 - Any unresolved variable names are global variables or expression aliases.
   - If global variable, the process is trivial.
   - If an expression alias, we have to inject the flattened version in place of the variable usage.
     - We must forbid any control flow that doesn't stay within the range / return.
       For example, `goto`, `break`, `continue`, and `@label@` are not allowed as they don't operate "as sequential values" but rather non-trivially alter control flow.
       We probably want to treat `break` and `continue` as if they were unnamed goto labels.
       I think we also need to link them after macro resolution so that they will still work even if generated from macros.
*/

impl<'env> Executable<'env> for GetFuncBody<'env> {
    type Output = &'env FuncBody<'env>;

    #[allow(unused_variables)]
    #[allow(unreachable_code)]
    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let def = &self.workspace.symbols.all_funcs[self.func_ref];

        // Compute control flow graph
        let cfg = match &mut self.cfg {
            Some(value) => *value,
            None => self.cfg.insert(
                ctx.alloc(
                    flatten_func_ignore_const_evals(
                        &def.head.params,
                        def.stmts.clone(),
                        def.head.source,
                    )
                    .finalize_gotos()?,
                ),
            ),
        };
        cfg.write_to_graphviz_file("oops.dot");

        // Compute immediate dominators
        let (dominators, post_order) = match &mut self.dominators_and_post_order {
            Some(value) => value,
            None => *self
                .dominators_and_post_order
                .insert(ctx.alloc(compute_idom_tree(&cfg))),
        };

        // Acquire settings and configuration
        let settings =
            &self.workspace.settings[def.settings.expect("settings assigned for function")];
        let c_integer_assumptions = settings.c_integer_assumptions();

        // Annotate each CFG node (may suspend)
        while self.num_processed < cfg.nodes.len() {
            let node_ref = *post_order
                .get(post_order.len() - 1 - self.num_processed)
                .unwrap();
            let node = &cfg.nodes[node_ref];

            self.types.insert(
                node_ref.into_raw(),
                match &node.kind {
                    NodeKind::Start(_) => Resolved::void(node.source),
                    NodeKind::Sequential(sequential_node) => match &sequential_node.kind {
                        SequentialNodeKind::Join1(incoming) => self.get_typed(*incoming).clone(),
                        SequentialNodeKind::JoinN(items, conform_behavior) => {
                            if let Some(conform_behavior) = conform_behavior {
                                let edges = items
                                    .iter()
                                    .map(|(_, incoming)| Value::new(*incoming))
                                    .collect_vec();

                                let Some(unified) = unify_types(
                                    None,
                                    edges.iter().map(|value| value.ty(&self.types)),
                                    *conform_behavior,
                                    node.source,
                                ) else {
                                    // TODO: Improve error message
                                    return Err(ErrorDiagnostic::new(
                                        format!("Inconsistent types from incoming blocks"),
                                        node.source,
                                    )
                                    .into());
                                };

                                Resolved::from_type(unified)
                            } else {
                                Resolved::void(node.source)
                            }
                        }
                        SequentialNodeKind::Const(_) => {
                            // For this case, we will have to suspend until the result
                            // of the calcuation is complete
                            unimplemented!("SequentialNodeKind::Const not supported yet")
                        }
                        SequentialNodeKind::Name(needle) => {
                            let var_ty = if let Some(var_ty) = executor.demand(self.dominator_type)
                            {
                                var_ty.clone()
                            } else {
                                let mut dominator_ref = *dominators
                                    .get(node_ref.into_raw())
                                    .expect("dominator to exist");
                                let mut var_ty = None::<Type<'env>>;

                                while dominator_ref != cfg.start() {
                                    let dom = &cfg.nodes[dominator_ref];

                                    match &dom.kind {
                                        NodeKind::Sequential(sequential_node) => {
                                            match &sequential_node.kind {
                                                SequentialNodeKind::Parameter(name, ty, _) => {
                                                    if needle.as_plain_str() == Some(name) {
                                                        return suspend!(
                                                            self.dominator_type,
                                                            executor.request(ResolveType::new(
                                                                &self.workspace,
                                                                ty,
                                                                &self.decl_scope
                                                            )),
                                                            ctx
                                                        );
                                                    }
                                                }
                                                SequentialNodeKind::Declare(name, ty, idx) => {
                                                    if needle.as_plain_str() == Some(name) {
                                                        return suspend!(
                                                            self.dominator_type,
                                                            executor.request(ResolveType::new(
                                                                &self.workspace,
                                                                ty,
                                                                &self.decl_scope
                                                            )),
                                                            ctx
                                                        );
                                                    }
                                                }
                                                SequentialNodeKind::DeclareAssign(name, value) => {
                                                    if needle.as_plain_str() == Some(name) {
                                                        var_ty = Some(
                                                            self.get_typed(dominator_ref)
                                                                .ty()
                                                                .clone(),
                                                        );
                                                        break;
                                                    }
                                                }
                                                _ => (),
                                            }
                                        }
                                        _ => (),
                                    }

                                    dominator_ref =
                                        *dominators.get(dominator_ref.into_raw()).unwrap();
                                }

                                let Some(var_ty) = var_ty else {
                                    return Err(ErrorDiagnostic::new(
                                        format!("Undefined variable '{}'", needle),
                                        node.source,
                                    )
                                    .into());
                                };

                                var_ty
                            };

                            // Reset dominator type for next node that needs it
                            self.dominator_type = None;

                            eprintln!("Variable {} has type {:?}", needle, var_ty);
                            Resolved::from_type(var_ty)
                        }
                        SequentialNodeKind::Parameter(_, _, _) => Resolved::void(node.source),
                        SequentialNodeKind::Declare(_, _, _) => Resolved::void(node.source),
                        SequentialNodeKind::Assign(_, _) => Resolved::void(node.source),
                        SequentialNodeKind::BinOp(left, bin_op, right) => {
                            let left_ty = self.get_typed(*left).ty();
                            let right_ty = self.get_typed(*right).ty();

                            if let (
                                TypeKind::IntegerLiteral(left),
                                TypeKind::IntegerLiteral(right),
                            ) = (&left_ty.kind, &right_ty.kind)
                            {
                                resolve_basic_binary_operation_expr_on_literals(
                                    bin_op,
                                    &left,
                                    &right,
                                    node.source,
                                )
                                .map_err(Continuation::Error)?
                            } else {
                                /*
                                // TODO: This should be determined by the binary operator
                                let conform_behavior = ConformBehavior::Adept(c_integer_assumptions);

                                let unified_type = unify_types(
                                    preferred_type,
                                    [left_ty, right_ty].into_iter(),
                                    conform_behavior,
                                    node.source,
                                );
                                */

                                /*
                                    let unified_type = unify_types(
                                        ctx,
                                        preferred_type.map(|preferred_type| preferred_type.view(ctx.asg)),
                                        &mut [&mut left, &mut right],
                                        ctx.adept_conform_behavior(),
                                        source,
                                    )
                                    .ok_or_else(|| {
                                        ResolveErrorKind::IncompatibleTypesForBinaryOperator {
                                            operator: binary_operation.operator.to_string(),
                                            left: left.ty.to_string(),
                                            right: right.ty.to_string(),
                                        }
                                        .at(source)
                                    })?;

                                    let operator =
                                        resolve_basic_binary_operator(ctx, &binary_operation.operator, &unified_type, source)?;

                                    let result_type = if binary_operation.operator.returns_boolean() {
                                        asg::TypeKind::Boolean.at(source)
                                    } else {
                                        unified_type
                                    };

                                    Ok(TypedExpr::new(
                                        result_type,
                                        asg::Expr::new(
                                            asg::ExprKind::BasicBinaryOperation(Box::new(asg::BasicBinaryOperation {
                                                operator,
                                                left,
                                                right,
                                            })),
                                            source,
                                        ),
                                    ))
                                */

                                unimplemented!("BinOp non-constant")
                            }
                        }
                        SequentialNodeKind::Boolean(value) => {
                            Resolved::from_type(TypeKind::BooleanLiteral(*value).at(node.source))
                        }
                        SequentialNodeKind::Integer(integer) => {
                            let source = node.source;

                            let ty = match integer {
                                ast::Integer::Known(known) => {
                                    TypeKind::from(known.as_ref()).at(source)
                                }
                                ast::Integer::Generic(value) => {
                                    TypeKind::IntegerLiteral(value.clone()).at(source)
                                }
                            };

                            Resolved::from_type(ty)
                        }
                        SequentialNodeKind::Float(_) => todo!("Float"),
                        SequentialNodeKind::AsciiChar(_) => todo!("AsciiChar"),
                        SequentialNodeKind::Utf8Char(_) => todo!("Utf8Char"),
                        SequentialNodeKind::String(_) => todo!("String"),
                        SequentialNodeKind::NullTerminatedString(cstring) => {
                            let char = TypeKind::CInteger(CInteger::Char, None).at(node.source);
                            Resolved::from_type(TypeKind::Ptr(ctx.alloc(char)).at(node.source))
                        }
                        SequentialNodeKind::Null => todo!("Null"),
                        SequentialNodeKind::Void => Resolved::void(node.source),
                        SequentialNodeKind::Call(node_call) => {
                            todo!("get_func_body Call")
                            // let found = find_or_suspend();

                            // let all_conformed = existing_conformed.conform_rest(rest).or_suspend();

                            // result
                        }
                        SequentialNodeKind::DeclareAssign(_name, value) => conform_to_default(
                            self.get_typed(*value).ty(),
                            c_integer_assumptions,
                            self.builtin_types,
                        )?,
                        SequentialNodeKind::Member(idx, _, privacy) => todo!("Member"),
                        SequentialNodeKind::ArrayAccess(idx, idx1) => todo!("ArrayAccess"),
                        SequentialNodeKind::StructLiteral(node_struct_literal) => {
                            todo!("StructLiteral")
                        }
                        SequentialNodeKind::UnaryOperation(unary_operator, idx) => {
                            todo!("UnaryOperation")
                        }
                        SequentialNodeKind::StaticMemberValue(static_member_value) => {
                            todo!("StaticMemberValue")
                        }
                        SequentialNodeKind::StaticMemberCall(node_static_member_call) => {
                            todo!("StaticMemberCall")
                        }
                        SequentialNodeKind::SizeOf(_) => todo!("SizeOf"),
                        SequentialNodeKind::SizeOfValue(idx) => todo!("SizeOfValue"),
                        SequentialNodeKind::InterpreterSyscall(node_interpreter_syscall) => {
                            todo!("InterpreterSyscall")
                        }
                        SequentialNodeKind::IntegerPromote(idx) => todo!("IntegerPromote"),
                        SequentialNodeKind::StaticAssert(idx, _) => todo!("StaticAssert"),
                        SequentialNodeKind::ConformToBool(idx, language) => {
                            if let ast::Language::Adept = language {
                                Resolved::from_type(self.builtin_types.bool.clone())
                            } else {
                                todo!("ConformToBool")
                            }
                        }
                        SequentialNodeKind::Is(idx, _) => unimplemented!("Is"),
                        SequentialNodeKind::DirectGoto(label_value) => Resolved::never(node.source),
                        SequentialNodeKind::LabelLiteral(name) => {
                            return Err(ErrorDiagnostic::new(
                                "Indirect goto labels are not supported yet",
                                node.source,
                            )
                            .into());
                        }
                    },
                    NodeKind::Branching(branch_node) => Resolved::void(node.source),
                    NodeKind::Terminating(terminating_node) => Resolved::void(node.source),
                    NodeKind::Scope(_) => Resolved::never(node.source),
                },
            );

            self.num_processed += 1;
        }

        Err(ErrorDiagnostic::new(
            "Computing function bodies is not implemented yet!",
            def.head.source,
        )
        .into())

        // Ok(ctx.alloc(todo!("compute func body")))
    }
}
