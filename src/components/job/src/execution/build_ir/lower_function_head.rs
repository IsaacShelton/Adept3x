use crate::{
    Continuation, Executable, ExecutionCtx, Executor, ir,
    module_graph::ModuleView,
    repr::{Compiler, FuncHead},
};
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct LowerFunctionHead<'env> {
    view: ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    head: ByAddress<&'env FuncHead<'env>>,
}

impl<'env> LowerFunctionHead<'env> {
    pub fn new(
        view: ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        head: &'env FuncHead<'env>,
    ) -> Self {
        Self {
            view,
            compiler: ByAddress(compiler),
            head: ByAddress(head),
        }
    }
}

impl<'env> Executable<'env> for LowerFunctionHead<'env> {
    type Output = ();

    fn execute(
        self,
        _executor: &Executor<'env>,
        _ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let ir = self.view.web.graph(self.view.graph, |graph| graph.ir);

        let _ir_func_head_ref = ir.funcs.alloc(ir::Func {
            mangled_name: self.head.name,
            params: &[],
            return_type: ir::Type::Void,
            basicblocks: &[],
            is_cstyle_variadic: false,
            ownership: self.head.metadata.ownership,
            abide_abi: self.head.metadata.abi.is_c(),
        });

        Ok(())
    }
}
