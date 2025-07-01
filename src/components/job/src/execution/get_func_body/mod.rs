mod basic_bin_op;
mod compute_preferred_types;
mod dominators;

use super::Executable;
use crate::{
    BuiltinTypes, Continuation, ExecutionCtx, Executor, ResolveType, Resolved, ResolvedData,
    Suspend, SuspendMany, Value,
    cfg::{
        NodeId, NodeKind, NodeRef, SequentialNode, SequentialNodeKind, UntypedCfg,
        flatten_func_ignore_const_evals,
    },
    conform::conform_to_default,
    repr::{DeclScope, FuncBody, Type, TypeKind},
    sub_task::SubTask,
    unify::unify_types,
};
use arena::ArenaMap;
use ast::ConformBehavior;
use ast_workspace::{AstWorkspace, FuncRef};
use basic_bin_op::{
    resolve_basic_binary_operation_expr_on_literals, resolve_basic_binary_operator,
};
use by_address::ByAddress;
use compute_preferred_types::{ComputePreferredTypes, ComputePreferredTypesUserData};
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
    compute_preferred_types: ComputePreferredTypes<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_nodes: ArenaMap<NodeId, Resolved<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    num_resolved_nodes: usize,

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
            compute_preferred_types: ComputePreferredTypes::default(),
            resolved_nodes: ArenaMap::new(),
            num_resolved_nodes: 0,
            dominator_type: None,
            builtin_types,
            cfg: None,
            dominators_and_post_order: None,
        }
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

        // 1) Compute control flow graph
        let cfg = match self.cfg {
            Some(value) => value,
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

        // 2) Compute immediate dominators and post order traversal
        let (dominators, post_order) = self
            .dominators_and_post_order
            .get_or_insert_with(|| ctx.alloc(compute_idom_tree(&cfg)));

        // 3) Acquire settings and configuration
        let settings =
            &self.workspace.settings[def.settings.expect("settings assigned for function")];
        let c_integer_assumptions = settings.c_integer_assumptions();

        // 4) Determine what type each CFG node prefers to be.
        let preferred_types = match self.compute_preferred_types.execute_sub_task(
            executor,
            ctx,
            ComputePreferredTypesUserData {
                post_order: post_order.as_slice(),
                cfg,
                func_return_type: &def.head.return_type,
                workspace: &self.workspace,
                decl_scope: &self.decl_scope,
                builtin_types: self.builtin_types,
            },
        ) {
            Ok(ok) => ok,
            Err(e) => return Err(e.map(|_| self.into()).into()),
        };

        // 5) Resolve types and linked data for each CFG node (may suspend)
        while self.num_resolved_nodes < post_order.len() {
            // We must process nodes in reverse post-order to ensure proper ordering, lexical
            // ordering is not enough! Consider `goto` for example.
            let node_ref = post_order[post_order.len() - 1 - self.num_resolved_nodes];
            let node = &cfg.nodes[node_ref];

            let get_typed = |node_ref: NodeRef| -> &Resolved<'env> {
                return self.resolved_nodes.get(node_ref.into_raw()).unwrap();
            };

            let resolved_node = match &node.kind {
                NodeKind::Start(_) => Resolved::void(node.source),
                NodeKind::Sequential(sequential_node) => match &sequential_node.kind {
                    SequentialNodeKind::Join1(incoming) => get_typed(*incoming).clone(),
                    SequentialNodeKind::JoinN(items, Some(conform_behavior)) => {
                        let edges = items
                            .iter()
                            .map(|(_, incoming)| Value::new(*incoming))
                            .collect_vec();

                        let Some(unified) = unify_types(
                            None,
                            edges
                                .iter()
                                .map(|value| value.ty(&self.resolved_nodes, self.builtin_types)),
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
                    }
                    SequentialNodeKind::JoinN(items, None) => Resolved::void(node.source),
                    SequentialNodeKind::Const(_) => {
                        // For this case, we will have to suspend until the result
                        // of the calcuation is complete
                        unimplemented!("SequentialNodeKind::Const not supported yet")
                    }
                    SequentialNodeKind::Name(needle) => {
                        let var_ty = if let Some(ty) = executor.demand(self.dominator_type) {
                            ty.clone()
                        } else {
                            let mut dominator_ref = *dominators
                                .get(node_ref.into_raw())
                                .expect("dominator to exist");

                            loop {
                                let dom = &cfg.nodes[dominator_ref];

                                match &dom.kind {
                                    NodeKind::Start(_) => {
                                        return Err(ErrorDiagnostic::new(
                                            format!("Undefined variable '{}'", needle),
                                            node.source,
                                        )
                                        .into());
                                    }
                                    NodeKind::Sequential(SequentialNode {
                                        kind: SequentialNodeKind::Parameter(name, ty, _),
                                        ..
                                    }) if needle.as_plain_str() == Some(name) => {
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
                                    NodeKind::Sequential(SequentialNode {
                                        kind: SequentialNodeKind::Declare(name, ty, idx),
                                        ..
                                    }) if needle.as_plain_str() == Some(name) => {
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
                                    NodeKind::Sequential(SequentialNode {
                                        kind: SequentialNodeKind::DeclareAssign(name, value),
                                        ..
                                    }) if needle.as_plain_str() == Some(name) => {
                                        break get_typed(dominator_ref).ty().clone();
                                    }
                                    _ => (),
                                }

                                dominator_ref = *dominators.get(dominator_ref.into_raw()).unwrap();
                            }
                        };

                        // Reset dominator type for next node that needs it
                        self.dominator_type = None;

                        eprintln!("Variable {} has type {:?}", needle, var_ty);
                        Resolved::from_type(var_ty)
                    }
                    SequentialNodeKind::Parameter(_, _, _) => Resolved::void(node.source),
                    SequentialNodeKind::Declare(_, _, _) => Resolved::void(node.source),
                    SequentialNodeKind::Assign(_, _) => Resolved::void(node.source),
                    SequentialNodeKind::BinOp(left, bin_op, right, language) => {
                        let left_ty = get_typed(*left).ty();
                        let right_ty = get_typed(*right).ty();

                        if let (TypeKind::IntegerLiteral(left), TypeKind::IntegerLiteral(right)) =
                            (&left_ty.kind, &right_ty.kind)
                        {
                            resolve_basic_binary_operation_expr_on_literals(
                                bin_op,
                                &left,
                                &right,
                                node.source,
                            )
                            .map_err(Continuation::Error)?
                        } else {
                            let conform_behavior = match language {
                                ast::Language::Adept => {
                                    ConformBehavior::Adept(c_integer_assumptions)
                                }
                                ast::Language::C => ConformBehavior::C,
                            };

                            let preferred_type = preferred_types.get(node_ref.into_raw()).copied();

                            let Some(unified_type) = unify_types(
                                preferred_type,
                                [left_ty, right_ty].into_iter(),
                                conform_behavior,
                                node.source,
                            ) else {
                                return Err(ErrorDiagnostic::new(
                                    format!(
                                        "Incompatible types '{}' and '{}' for '{}'",
                                        left_ty, right_ty, bin_op
                                    ),
                                    node.source,
                                )
                                .into());
                            };

                            let operator =
                                resolve_basic_binary_operator(bin_op, &unified_type, node.source)
                                    .map_err(Continuation::Error)?;

                            dbg!(&preferred_type, &unified_type, &operator);
                            let result_type = if bin_op.returns_boolean() {
                                self.builtin_types.bool.kind.clone().at(node.source)
                            } else {
                                unified_type
                            };

                            Resolved::new(result_type, ResolvedData::BasicBinaryOperator(operator))
                        }
                    }
                    SequentialNodeKind::Boolean(value) => {
                        Resolved::from_type(TypeKind::BooleanLiteral(*value).at(node.source))
                    }
                    SequentialNodeKind::Integer(integer) => {
                        let source = node.source;

                        let ty = match integer {
                            ast::Integer::Known(known) => TypeKind::from(known.as_ref()).at(source),
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
                        get_typed(*value).ty(),
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
                NodeKind::Terminating(terminating_node) => Resolved::never(node.source),
                NodeKind::Scope(_) => {
                    // When joining control flow paths, consider the path immediately
                    // from this "begin scope" node to diverge since it's not a real branch.
                    // Since it's only used for scoping, so don't take it into account
                    // during type unifying from multiple code paths.
                    Resolved::never(node.source)
                }
            };

            self.resolved_nodes
                .insert(node_ref.into_raw(), resolved_node);
            self.num_resolved_nodes += 1;
        }

        Err(ErrorDiagnostic::new(
            "Computing function bodies is not implemented yet!",
            def.head.source,
        )
        .into())

        // Ok(ctx.alloc(todo!("compute func body")))
    }
}
