use super::Executable;
use crate::{BumpAllocator, Continuation, Executor};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Print<'env>(pub &'env str);

impl<'env> Executable<'env> for Print<'env> {
    type Output = ();

    fn execute(
        self,
        _executor: &Executor<'env>,
        _allocator: &'env BumpAllocator,
    ) -> Result<Self::Output, Continuation<'env>> {
        println!("{}", self.0);
        Ok(())
    }
}
