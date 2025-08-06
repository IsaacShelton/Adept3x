use crate::{Continuation, Execution, ExecutionCtx, Executor};
use diagnostics::ErrorDiagnostic;

// NOTE: Sub tasks are responsible for caching their own results
// in the most efficient way for their own particular case.
// Once Ok(...) is returned, the same Ok(...) value must be returned
// each time thereafter.
pub trait SubTask<'env>
where
    Self: Sized,
{
    type SubArtifact<'a>
    where
        Self: 'a,
        'env: 'a;

    type UserData<'a>
    where
        Self: 'a,
        'env: 'a;

    #[must_use]
    fn execute_sub_task<'a, 'ctx>(
        &'a mut self,
        executor: &'a Executor<'env>,
        ctx: &'ctx mut ExecutionCtx<'env>,
        user_data: Self::UserData<'a>,
    ) -> Result<
        Self::SubArtifact<'a>,
        Result<impl Fn(Execution<'env>) -> Continuation<'env> + 'static, ErrorDiagnostic>,
    >;
}
