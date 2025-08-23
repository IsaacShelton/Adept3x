use crate::{Continuation, Executable, ExecutionCtx, Executor};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Print<'env>(pub &'env str);

impl<'env> Executable<'env> for Print<'env> {
    type Output = ();

    fn execute(
        self,
        _executor: &Executor<'env>,
        _ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        println!("{}", self.0);
        Ok(())
    }
}
