use crate::{
    Continuation, Execution, ExecutionCtx, Executor, Suspend,
    execution::lower::{LowerType, function_body::ir_builder::IrBuilder},
    ir,
    module_graph::ModuleView,
    repr::{Compiler, Type, TypeKind},
    sub_task::SubTask,
};
use diagnostics::ErrorDiagnostic;

#[derive(Clone)]
pub struct DerefDest<'env> {
    view: &'env ModuleView<'env>,
    compiler: &'env Compiler<'env>,
    ty: &'env Type<'env>,
    dest: ir::Value<'env>,
    lowered_type: Suspend<'env, ir::Type<'env>>,
}

impl<'env> DerefDest<'env> {
    pub fn new(
        view: &'env ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        dest: ir::Value<'env>,
        ty: &'env Type<'env>,
    ) -> Self {
        Self {
            view,
            compiler,
            dest,
            ty,
            lowered_type: None,
        }
    }
}

impl<'env> SubTask<'env> for DerefDest<'env> {
    type SubArtifact<'a>
        = (ir::Value<'env>, ir::Type<'env>)
    where
        Self: 'a,
        'env: 'a;

    type UserData<'a>
        = &'a mut IrBuilder<'env>
    where
        Self: 'a,
        'env: 'a;

    fn execute_sub_task<'a, 'ctx>(
        &'a mut self,
        executor: &'a Executor<'env>,
        ctx: &'ctx mut ExecutionCtx<'env>,
        builder: Self::UserData<'a>,
    ) -> Result<
        Self::SubArtifact<'a>,
        Result<impl FnOnce(Execution<'env>) -> Continuation<'env> + 'env, ErrorDiagnostic>,
    > {
        loop {
            let TypeKind::Deref(nested) = &self.ty.kind else {
                break;
            };

            let TypeKind::Deref(_) = &nested.kind else {
                break;
            };

            let Some(lowered_type) = executor.demand(self.lowered_type) else {
                return suspend_from_subtask!(
                    self.lowered_type,
                    executor.request(LowerType::new(self.view, self.compiler, nested)),
                    ctx
                );
            };

            self.dest = builder.push(ir::Instr::Load {
                pointer: self.dest,
                pointee: lowered_type,
            });
            self.lowered_type = None;
            self.ty = nested;
        }

        let Some(lowered_type) = executor.demand(self.lowered_type) else {
            return suspend_from_subtask!(
                self.lowered_type,
                executor.request(LowerType::new(&self.view, &self.compiler, &self.ty)),
                ctx
            );
        };

        Ok((self.dest, lowered_type))
    }
}
