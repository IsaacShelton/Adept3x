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

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    current_basicblock: usize,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    current_node_index: usize,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    basicblocks: Vec<Vec<ir::Instr<'env>>>,
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
            current_basicblock: 0,
            current_node_index: 0,
            basicblocks: vec![vec![]],
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

        // This may be easier if the CFG representation is already in basicblocks...
        // Otherwise, we have to convert it to basicblocks here anyway...

        // We can then also make variable lookup way faster by using hashmaps...
        // Possibly either by using a "time" score of which is declared at each time
        // within a basicblock, or just by having basicblocks be processed in reverse
        // post order, which I think we already do, and from top to bottom.

        // That would also greatly speed up the time taken to compute the
        // immediate dominators tree too I think.

        // We would probably want to keep all control-flow modifying constructs
        // within the resolution stage anyway, so they can correctly impact
        // the control-flow sensitive type checking.

        // TODO: Here is where we will do monomorphization (but only for the function body)...

        let ir_func_ref = self.func;

        let basicblocks = todo!();

        let ir_func = &ir.funcs[ir_func_ref];
        ir_func.basicblocks.set(basicblocks).unwrap();

        todo!(
            "lower function body {:?} {:?} {:?}",
            func,
            self.head,
            self.body
        )
    }
}
