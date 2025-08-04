use crate::{
    BuiltinTypes, ExecutionCtx, Executor, ResolveTypeKeepAliases, Suspend,
    cfg::{
        NodeId, NodeKind, NodeRef, SequentialNode, SequentialNodeKind, TerminatingNode, UntypedCfg,
    },
    module_graph::ModuleView,
    repr::Type,
    sub_task::SubTask,
};
use arena::ArenaMap;
use ast::{UnaryMathOperator, UnaryOperator};
use ast_workspace::AstWorkspace;

#[derive(Debug)]
pub struct ComputePreferredTypesUserData<'env, 'a> {
    pub post_order: &'a [NodeRef],
    pub cfg: &'env UntypedCfg,
    pub func_return_type: &'env ast::Type,
    pub workspace: &'env AstWorkspace<'env>,
    pub view: ModuleView<'env>,
    pub builtin_types: &'env BuiltinTypes<'env>,
}

#[derive(Clone, Debug, Default)]
pub struct ComputePreferredTypes<'env> {
    preferred_types: Option<ArenaMap<NodeId, &'env Type<'env>>>,
    num_preferred_processed: usize,
    waiting_on_type: Suspend<'env, &'env Type<'env>>,
}

impl<'env> SubTask<'env> for ComputePreferredTypes<'env> {
    type SubArtifact<'a>
        = &'a ArenaMap<NodeId, &'env Type<'env>>
    where
        Self: 'a;

    type UserData<'a>
        = ComputePreferredTypesUserData<'env, 'a>
    where
        Self: 'a;

    fn execute_sub_task<'a, 'ctx>(
        &'a mut self,
        executor: &'a Executor<'env>,
        ctx: &'ctx mut ExecutionCtx<'env>,
        user_data: Self::UserData<'a>,
    ) -> Result<Self::SubArtifact<'a>, Result<(), diagnostics::ErrorDiagnostic>> {
        let cfg = user_data.cfg;
        let post_order = user_data.post_order;

        let preferred_types = self
            .preferred_types
            .get_or_insert_with(|| ArenaMap::with_capacity(cfg.nodes.len()));

        while self.num_preferred_processed < post_order.len() {
            let node_ref = post_order[self.num_preferred_processed];
            let node = &cfg.nodes[node_ref];

            match &node.kind {
                NodeKind::Sequential(SequentialNode {
                    kind: SequentialNodeKind::Declare(_, expected_var_ty, Some(value)),
                    ..
                }) => {
                    let Some(fulfilled) = self.waiting_on_type.take() else {
                        return sub_task_suspend!(
                            self,
                            waiting_on_type,
                            executor.request(ResolveTypeKeepAliases::new(
                                user_data.workspace,
                                expected_var_ty,
                                user_data.view,
                            )),
                            ctx
                        );
                    };

                    preferred_types
                        .insert(value.into_raw(), executor.demand(Some(fulfilled)).unwrap());
                }
                NodeKind::Sequential(SequentialNode {
                    kind: SequentialNodeKind::Join1(value),
                    ..
                }) => {
                    if let Some(preferred) = preferred_types.get(node_ref.into_raw()) {
                        preferred_types.insert(value.into_raw(), preferred);
                    }
                }
                NodeKind::Sequential(SequentialNode {
                    kind: SequentialNodeKind::JoinN(values, _),
                    ..
                }) => {
                    if let Some(preferred) = preferred_types.get(node_ref.into_raw()).copied() {
                        for (_, value) in values {
                            preferred_types.insert(value.into_raw(), preferred);
                        }
                    }
                }
                NodeKind::Sequential(SequentialNode {
                    kind: SequentialNodeKind::UnaryOperation(op, value),
                    ..
                }) => match op {
                    UnaryOperator::Math(
                        UnaryMathOperator::Not
                        | UnaryMathOperator::BitComplement
                        | UnaryMathOperator::Negate,
                    ) => {
                        if let Some(preferred) = preferred_types.get(node_ref.into_raw()).copied() {
                            preferred_types.insert(value.into_raw(), preferred);
                        }
                    }
                    UnaryOperator::Math(UnaryMathOperator::IsNonZero) => (),
                    UnaryOperator::AddressOf => todo!(),
                    UnaryOperator::Dereference => todo!(),
                },
                NodeKind::Sequential(SequentialNode {
                    kind: SequentialNodeKind::BinOp(left, op, right, _),
                    ..
                }) => {
                    if !op.returns_boolean() {
                        if let Some(preferred) = preferred_types.get(node_ref.into_raw()).copied() {
                            preferred_types.insert(left.into_raw(), preferred);
                            preferred_types.insert(right.into_raw(), preferred);
                        }
                    }
                }
                NodeKind::Sequential(..) => (),
                NodeKind::Terminating(TerminatingNode::Return(Some(value))) => {
                    let Some(fulfilled) = self.waiting_on_type.take() else {
                        return sub_task_suspend!(
                            self,
                            waiting_on_type,
                            executor.request(ResolveTypeKeepAliases::new(
                                user_data.workspace,
                                user_data.func_return_type,
                                user_data.view,
                            )),
                            ctx
                        );
                    };

                    preferred_types
                        .insert(value.into_raw(), executor.demand(Some(fulfilled)).unwrap());
                }
                NodeKind::Branching(branch) => {
                    preferred_types
                        .insert(branch.condition.into_raw(), &user_data.builtin_types.bool);
                }
                NodeKind::Start(_)
                | NodeKind::Scope(_)
                | NodeKind::Terminating(
                    TerminatingNode::Break
                    | TerminatingNode::Continue
                    | TerminatingNode::Computed(_)
                    | TerminatingNode::Return(None),
                ) => (),
            };

            self.num_preferred_processed += 1;
        }

        Ok(preferred_types)
    }
}
