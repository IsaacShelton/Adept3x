mod dominators;

use super::Executable;
use crate::{
    Continuation, ExecutionCtx, Executor, SuspendMany, Typed, Value,
    repr::{DeclScope, FuncBody, Type, TypeKind},
    unify::unify_types,
};
use arena::Id;
use ast::{NodeId, NodeKind, NodeRef, SequentialNodeKind};
use ast_workspace::{AstWorkspace, FuncRef};
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
                        let mut dominator_ref = *dominators.get(node_ref.into_raw()).unwrap();
                        let mut var_ty = None::<Type<'env>>;

                        while dominator_ref != cfg.start() {
                            let dom = &cfg.ordered_nodes[dominator_ref];

                            match &dom.kind {
                                NodeKind::Sequential(sequential_node) => {
                                    match &sequential_node.kind {
                                        SequentialNodeKind::NewVariable(name, ty) => {
                                            if needle.as_plain_str() == Some(name) {
                                                todo!("resolve variable ty for non-declare-assign");
                                                // var_ty = Some(ty.clone());
                                                break;
                                            }
                                        }
                                        SequentialNodeKind::Declare(name, ty, idx) => {
                                            if needle.as_plain_str() == Some(name) {
                                                todo!("resolve variable ty for non-declare-assign");
                                                // var_ty = Some(ty.clone());
                                                break;
                                            }
                                        }
                                        SequentialNodeKind::DeclareAssign(name, value) => {
                                            if needle.as_plain_str() == Some(name) {
                                                var_ty = Some(self.get_typed(*value).ty().clone());
                                                break;
                                            }
                                        }
                                        _ => (),
                                    }
                                }
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

                        todo!("Name {:?}", var_ty);
                    }
                    SequentialNodeKind::OpenScope => Typed::void(node.source),
                    SequentialNodeKind::CloseScope => Typed::void(node.source),
                    SequentialNodeKind::NewVariable(_, _) => todo!("NewVariable"),
                    SequentialNodeKind::Declare(_, _, idx) => todo!("Declare"),
                    SequentialNodeKind::Assign(idx, idx1) => todo!("Assign"),
                    SequentialNodeKind::BinOp(idx, basic_binary_operator, idx1) => todo!("BinOp"),
                    SequentialNodeKind::Boolean(_) => todo!("Boolean"),
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
