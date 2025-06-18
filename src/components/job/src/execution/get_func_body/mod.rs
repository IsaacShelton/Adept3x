mod basic_bin_op;
mod dominators;

use super::Executable;
use crate::{
    Continuation, ExecutionCtx, Executor, ResolveType, Suspend, SuspendMany, Typed, Value,
    repr::{DeclScope, FuncBody, Type, TypeKind},
    unify::unify_types,
};
use arena::Id;
use ast::{NodeId, NodeKind, NodeRef, SequentialNodeKind};
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
    types: Vec<Typed<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    dominator_type: Suspend<'env, &'env Type<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    dominator: Option<NodeRef>,
}

impl<'env> GetFuncBody<'env> {
    pub fn new(
        workspace: &'env AstWorkspace<'env>,
        func_ref: FuncRef,
        decl_scope: &'env DeclScope,
    ) -> Self {
        Self {
            workspace: ByAddress(workspace),
            func_ref,
            decl_scope: ByAddress(decl_scope),
            inner_types: None,
            types: Vec::new(),
            dominator: None,
            dominator_type: None,
        }
    }

    fn get_typed(&self, node_ref: NodeRef) -> &Typed<'env> {
        &self.types[node_ref.into_raw().into_usize()]
    }
}

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
        let cfg = &def.cfg;

        let dominators = compute_idom_tree(cfg);

        if self.types.capacity() == 0 {
            self.types.reserve_exact(cfg.ordered_nodes.len());
        }

        while self.types.len() < cfg.ordered_nodes.len() {
            let node_index = self.types.len();
            let node_ref = unsafe { NodeRef::from_raw(NodeId::from_usize(node_index)) };
            let node = &cfg.ordered_nodes[node_ref];

            self.types.push(match &node.kind {
                NodeKind::Start(_) => Typed::void(node.source),
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

                            Typed::from_type(unified)
                        } else {
                            Typed::void(node.source)
                        }
                    }
                    SequentialNodeKind::Const(_) => {
                        // For this case, we will have to suspend until the result
                        // of the calcuation is complete
                        unimplemented!("SequentialNodeKind::Const not supported yet")
                    }
                    SequentialNodeKind::Name(needle) => {
                        let var_ty = if let Some(var_ty) = executor.demand(self.dominator_type) {
                            var_ty.clone()
                        } else {
                            let mut dominator_ref = *dominators.get(node_ref.into_raw()).unwrap();
                            let mut var_ty = None::<Type<'env>>;

                            while dominator_ref != cfg.start() {
                                let dom = &cfg.ordered_nodes[dominator_ref];

                                match &dom.kind {
                                    NodeKind::Sequential(sequential_node) => match &sequential_node
                                        .kind
                                    {
                                        SequentialNodeKind::NewVariable(name, ty) => {
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
                                                var_ty = Some(self.get_typed(*value).ty().clone());
                                                break;
                                            }
                                        }
                                        _ => (),
                                    },
                                    _ => (),
                                }

                                dominator_ref = *dominators.get(dominator_ref.into_raw()).unwrap();
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

                        eprintln!("Variable {} has type {:?}", needle, var_ty.kind);
                        Typed::from_type(var_ty)
                    }
                    SequentialNodeKind::OpenScope => Typed::void(node.source),
                    SequentialNodeKind::CloseScope => Typed::void(node.source),
                    SequentialNodeKind::NewVariable(_, _) => todo!("NewVariable"),
                    SequentialNodeKind::Declare(_, _, _) => Typed::void(node.source),
                    SequentialNodeKind::Assign(_, _) => Typed::void(node.source),
                    SequentialNodeKind::BinOp(left, bin_op, right) => {
                        let left_ty = self.get_typed(*left).ty();
                        let right_ty = self.get_typed(*right).ty();

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
                        Typed::from_type(TypeKind::BooleanLiteral(*value).at(node.source))
                    }
                    SequentialNodeKind::Integer(integer) => {
                        let source = node.source;

                        let ty = match integer {
                            ast::Integer::Known(known) => TypeKind::from(known.as_ref()).at(source),
                            ast::Integer::Generic(value) => {
                                TypeKind::IntegerLiteral(value.clone()).at(source)
                            }
                        };

                        Typed::from_type(ty)
                    }
                    SequentialNodeKind::Float(_) => todo!("Float"),
                    SequentialNodeKind::AsciiChar(_) => todo!("AsciiChar"),
                    SequentialNodeKind::Utf8Char(_) => todo!("Utf8Char"),
                    SequentialNodeKind::String(_) => todo!("String"),
                    SequentialNodeKind::NullTerminatedString(cstring) => {
                        let char = TypeKind::CInteger(CInteger::Char, None).at(node.source);
                        Typed::from_type(TypeKind::Ptr(ctx.alloc(char)).at(node.source))
                    }
                    SequentialNodeKind::Null => todo!("Null"),
                    SequentialNodeKind::Void => Typed::void(node.source),
                    SequentialNodeKind::Call(node_call) => {
                        todo!()
                        // let found = find_or_suspend();

                        // let all_conformed = existing_conformed.conform_rest(rest).or_suspend();

                        // result
                    }
                    SequentialNodeKind::DeclareAssign(_name, value) => {
                        self.get_typed(*value).clone()
                    }
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
                    SequentialNodeKind::ConformToBool(idx, language) => todo!("ConformToBool"),
                    SequentialNodeKind::Is(idx, _) => unimplemented!("Is"),
                },
                NodeKind::Branching(branch_node) => Typed::void(node.source),
                NodeKind::Terminating(terminating_node) => Typed::void(node.source),
            });
        }

        Err(ErrorDiagnostic::new(
            "Computing function bodies is not implemented yet!",
            def.head.source,
        )
        .into())

        // Ok(ctx.alloc(todo!("compute func body")))
    }
}
