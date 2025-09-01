use super::post_order_iter::PostOrderIterWithEnds;
use crate::{
    BasicBlockId, BuiltinTypes, CfgBuilder, Continuation, EndInstrKind, Execution, ExecutionCtx,
    Executor, InstrKind, InstrRef, Suspend, execution::resolve::ResolveType,
    module_graph::ModuleView, repr::UnaliasedType, sub_task::SubTask,
};
use arena::Idx;
use ast::{UnaryMathOperator, UnaryOperator};
use diagnostics::ErrorDiagnostic;
use std::marker::PhantomData;

#[allow(unused)]
#[derive(Debug)]
pub struct ComputePreferredTypesUserData<'env, 'a> {
    pub post_order: &'a [BasicBlockId],
    pub cfg: &'a mut CfgBuilder<'env>,
    pub func_return_type: &'env ast::Type,
    pub view: ModuleView<'env>,
    pub builtin_types: &'env BuiltinTypes<'env>,
}

#[derive(Clone, Debug, Default)]
pub struct ComputePreferredTypes<'env> {
    phantom: PhantomData<&'env ()>,
    post_order_iter: PostOrderIterWithEnds,
    waiting_on_type: Suspend<'env, UnaliasedType<'env>>,
    current: Option<InstrRef>,
}

impl<'env> SubTask<'env> for ComputePreferredTypes<'env> {
    type SubArtifact<'a>
        = ()
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
    ) -> Result<
        Self::SubArtifact<'a>,
        Result<impl FnOnce(Execution<'env>) -> Continuation<'env> + 'static, ErrorDiagnostic>,
    > {
        let cfg = user_data.cfg;
        let post_order = user_data.post_order;

        if self.current.is_none() {
            self.current = self.post_order_iter.next(cfg, post_order);
        }

        while let Some(instr_ref) = self.current {
            let bb = &cfg.basicblocks[unsafe { Idx::from_raw(instr_ref.basicblock) }];
            assert!(instr_ref.instr_or_end as usize <= bb.instrs.len());

            // If this is an end instruction, handle it differently...
            if instr_ref.instr_or_end as usize == bb.instrs.len() {
                match &bb.end.as_ref().unwrap().kind {
                    EndInstrKind::Return(Some(value)) => {
                        let Some(fulfilled) = self.waiting_on_type.take() else {
                            return suspend_from_subtask!(
                                self,
                                waiting_on_type,
                                executor.request(ResolveType::new(
                                    user_data.view,
                                    user_data.func_return_type,
                                )),
                                ctx
                            );
                        };

                        cfg.set_preferred_type(*value, executor.demand(Some(fulfilled)).unwrap());
                    }
                    EndInstrKind::Branch(condition, _, _, _) => {
                        cfg.set_preferred_type(*condition, user_data.builtin_types.bool());
                    }
                    EndInstrKind::IncompleteGoto(_)
                    | EndInstrKind::IncompleteBreak
                    | EndInstrKind::IncompleteContinue
                    | EndInstrKind::Return(None)
                    // NOTE: PHI nodes handle the propagation of preferred types
                    // for chosen values, not the branches/jumps themselves.
                    | EndInstrKind::Jump(_)
                    | EndInstrKind::NewScope(..)
                    | EndInstrKind::Unreachable => (),
                }

                self.current = self.post_order_iter.next(cfg, post_order);
                continue;
            }

            // Otherwise, this is a normal sequential instruction...
            let instr = &bb.instrs[instr_ref.instr_or_end as usize];

            match &instr.kind {
                InstrKind::Declare(_, _, None) => (),
                InstrKind::Declare(_, expected_var_ty, Some(value)) => {
                    let Some(fulfilled) = self.waiting_on_type.take() else {
                        return suspend_from_subtask!(
                            self,
                            waiting_on_type,
                            executor.request(ResolveType::new(user_data.view, expected_var_ty)),
                            ctx
                        );
                    };

                    cfg.set_preferred_type(*value, executor.demand(Some(fulfilled)).unwrap());
                }
                InstrKind::Phi(incoming, _) => {
                    if let Some(preferred) = instr.preferred_type {
                        for (_, value) in incoming.iter() {
                            if let Some(value) = value {
                                cfg.set_preferred_type(*value, preferred);
                            }
                        }
                    }
                }
                InstrKind::UnaryOperation(
                    UnaryOperator::Math(
                        UnaryMathOperator::Not
                        | UnaryMathOperator::BitComplement
                        | UnaryMathOperator::Negate,
                    ),
                    value,
                ) => {
                    if let Some(preferred) = instr.preferred_type {
                        cfg.set_preferred_type(*value, preferred);
                    }
                }
                InstrKind::UnaryOperation(
                    UnaryOperator::Math(UnaryMathOperator::IsNonZero)
                    | UnaryOperator::AddressOf
                    | UnaryOperator::Dereference,
                    _,
                ) => (),
                InstrKind::BinOp(a, op, b, _) => {
                    if !op.returns_boolean() {
                        if let Some(preferred) = instr.preferred_type {
                            let a = *a;
                            let b = *b;
                            cfg.set_preferred_type(a, preferred);
                            cfg.set_preferred_type(b, preferred);
                        }
                    }
                }
                InstrKind::Assign(_, _) => {
                    // NOTE: We need to know type information for the
                    // destination in order to set the preferred type...

                    // We need to figure what to do in situations like this:
                    // `x = x := 1243`
                    // where the type of the destination depends
                    // on the evaluation of the source.
                    // Maybe we just don't have a preferred type for situations like this
                    // where the destination type depends on the source.
                    // But it would also be easy to have incorrect preferred
                    // types if not careful.

                    // In order to properly support something like this,
                    // we need to:
                    // 1) Determine the type of the destination
                    // without evaluating it.
                    // 2) Set the preferred type of the source to the destination type.
                    // 3) Evaluate the source.
                    // 4) Evaluate the destination.

                    // We also want to do it this way to avoid
                    // code like this holding locks longer than necessary:
                    // ```rust
                    // *x.lock().uwrap() = 1234;
                    // ```
                }
                InstrKind::StructLiteral(_) => {
                    // NOTE: We should set the preferred types for each field here...
                }
                InstrKind::Name(_)
                | InstrKind::Parameter(_, _, _)
                | InstrKind::BooleanLiteral(_)
                | InstrKind::IntegerLiteral(_)
                | InstrKind::FloatLiteral(_)
                | InstrKind::AsciiCharLiteral(_)
                | InstrKind::Utf8CharLiteral(_)
                | InstrKind::StringLiteral(_)
                | InstrKind::NullTerminatedStringLiteral(_)
                | InstrKind::NullLiteral
                | InstrKind::VoidLiteral
                | InstrKind::Call(_)
                | InstrKind::DeclareAssign(_, _)
                | InstrKind::Member(_, _, _)
                | InstrKind::ArrayAccess(_, _)
                | InstrKind::SizeOf(_, _)
                | InstrKind::SizeOfValue(_, _)
                | InstrKind::InterpreterSyscall(_)
                | InstrKind::IntegerPromote(_)
                | InstrKind::ConformToBool(_, _)
                | InstrKind::Is(_, _)
                | InstrKind::LabelLiteral(_) => (),
            };

            self.current = self.post_order_iter.next(cfg, post_order);
        }

        Ok(())
    }
}
