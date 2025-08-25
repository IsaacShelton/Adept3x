use crate::{
    Continuation, Executable, ExecutionCtx, Executor, ir,
    module_graph::ModuleView,
    repr::{Compiler, FuncBody, FuncHead},
};
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct LowerFunctionBody<'env> {
    view: ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    func: ir::FuncRef<'env>,
    head: ByAddress<&'env FuncHead<'env>>,
    body: ByAddress<&'env FuncBody<'env>>,
}

impl<'env> LowerFunctionBody<'env> {
    pub fn new(
        view: ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        func: ir::FuncRef<'env>,
        head: &'env FuncHead<'env>,
        body: &'env FuncBody<'env>,
    ) -> Self {
        Self {
            view,
            compiler: ByAddress(compiler),
            func,
            head: ByAddress(head),
            body: ByAddress(body),
        }
    }
}

impl<'env> Executable<'env> for LowerFunctionBody<'env> {
    type Output = ();

    fn execute(
        self,
        _executor: &Executor<'env>,
        _ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let ir = self.view.web.graph(self.view.graph, |graph| graph.ir);
        let func = &ir.funcs[self.func];

        // TODO: Here is where we will do monomorphization (but only for the function body)...

        todo!(
            "lower function body {:?} {:?} {:?}",
            func,
            self.head,
            self.body
        )
        // Ok(())
    }
}
