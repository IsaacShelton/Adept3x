use crate::{
    Continuation, Executable, ExecutionCtx, Executor, module_graph::ModuleView, repr::Compiler,
};
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct BuildIr<'env> {
    view: ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,
}

impl<'env> BuildIr<'env> {
    pub fn new(compiler: &'env Compiler<'env>, view: ModuleView<'env>) -> Self {
        Self {
            view,
            compiler: ByAddress(compiler),
        }
    }
}

impl<'env> Executable<'env> for BuildIr<'env> {
    type Output = ();

    fn execute(
        self,
        _executor: &Executor<'env>,
        _ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        Ok(())
    }
}
