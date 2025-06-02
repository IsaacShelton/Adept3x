use super::Executable;
use crate::{
    Continuation, ExecutionCtx, Executor, SuspendMany, Typed, Value,
    repr::{DeclScope, FuncBody, Type},
    unify::unify_types,
};
use arena::Id;
use ast::{NodeId, NodeKind, NodeRef, SequentialNodeKind};
use ast_workspace::{AstWorkspace, FuncRef};
use by_address::ByAddress;
use derivative::Derivative;
use diagnostics::ErrorDiagnostic;
use itertools::Itertools;

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
                    SequentialNodeKind::Join2(_, incoming_a, _, incoming_b, conform_behavior) => {
                        if let Some(conform_behavior) = conform_behavior {
                            let edges: [_; 2] = [Value::new(*incoming_a), Value::new(*incoming_b)];

                            let Some(unified) = unify_types(
                                None,
                                edges.iter().map(|value| value.ty(&self.types)),
                                *conform_behavior,
                                node.source,
                            ) else {
                                return Err(ErrorDiagnostic::new(
                                    format!(
                                        "Incompatible types '{}' and '{}'",
                                        edges[0].ty(&self.types),
                                        edges[1].ty(&self.types)
                                    ),
                                    node.source,
                                )
                                .into());
                            };

                            Typed::from_type(unified)
                        } else {
                            Typed::void(node.source)
                        }
                    }
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
                    SequentialNodeKind::Name(name) => {
                        // We will probably have to keep a stack of some sort to keep
                        // track of which variables are still alive and/or refer to what
                        // and different points within the CFG, unless there is a
                        // smarter way to do it
                        todo!()
                    }
                    SequentialNodeKind::OpenScope => todo!(),
                    SequentialNodeKind::CloseScope => todo!(),
                    SequentialNodeKind::NewVariable(_, _) => todo!(),
                    SequentialNodeKind::Declare(_, _, idx) => todo!(),
                    SequentialNodeKind::Assign(idx, idx1) => todo!(),
                    SequentialNodeKind::BinOp(idx, basic_binary_operator, idx1) => todo!(),
                    SequentialNodeKind::Boolean(_) => todo!(),
                    SequentialNodeKind::Integer(integer) => todo!(),
                    SequentialNodeKind::Float(_) => todo!(),
                    SequentialNodeKind::AsciiChar(_) => todo!(),
                    SequentialNodeKind::Utf8Char(_) => todo!(),
                    SequentialNodeKind::String(_) => todo!(),
                    SequentialNodeKind::NullTerminatedString(cstring) => {
                        todo!("null terminated string is not implemented")
                    }
                    SequentialNodeKind::Null => todo!(),
                    SequentialNodeKind::Void => todo!(),
                    SequentialNodeKind::Call(node_call) => {
                        todo!()
                        // let found = find_or_suspend();

                        // let all_conformed = existing_conformed.conform_rest(rest).or_suspend();

                        // result
                    }
                    SequentialNodeKind::DeclareAssign(_, idx) => todo!(),
                    SequentialNodeKind::Member(idx, _, privacy) => todo!(),
                    SequentialNodeKind::ArrayAccess(idx, idx1) => todo!(),
                    SequentialNodeKind::StructLiteral(node_struct_literal) => todo!(),
                    SequentialNodeKind::UnaryOperation(unary_operator, idx) => todo!(),
                    SequentialNodeKind::StaticMemberValue(static_member_value) => todo!(),
                    SequentialNodeKind::StaticMemberCall(node_static_member_call) => todo!(),
                    SequentialNodeKind::SizeOf(_) => todo!(),
                    SequentialNodeKind::SizeOfValue(idx) => todo!(),
                    SequentialNodeKind::InterpreterSyscall(node_interpreter_syscall) => {
                        todo!()
                    }
                    SequentialNodeKind::IntegerPromote(idx) => todo!(),
                    SequentialNodeKind::StaticAssert(idx, _) => todo!(),
                    SequentialNodeKind::ConformToBool(idx, language) => todo!(),
                    SequentialNodeKind::Is(idx, _) => unimplemented!("SequentialNodeKind::Is"),
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
