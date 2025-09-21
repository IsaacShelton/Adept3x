use crate::{
    Continuation, Executable, ExecutionCtx, Executor, SuspendMany,
    execution::lower::LowerType,
    ir,
    module_graph::ModuleView,
    repr::{Compiler, FuncHead},
};
use by_address::ByAddress;
use derivative::Derivative;
use std::{
    hash::{DefaultHasher, Hasher},
    sync::OnceLock,
};

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct LowerFunctionHead<'env> {
    view: &'env ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    head: ByAddress<&'env FuncHead<'env>>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    inner_types: SuspendMany<'env, ir::Type<'env>>,
}

impl<'env> LowerFunctionHead<'env> {
    pub fn new(
        view: &'env ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        head: &'env FuncHead<'env>,
    ) -> Self {
        Self {
            view,
            compiler: ByAddress(compiler),
            head: ByAddress(head),
            inner_types: None,
        }
    }
}

impl<'env> Executable<'env> for LowerFunctionHead<'env> {
    type Output = ir::FuncRef<'env>;

    fn execute(
        self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let ir = self.view.web.graph(self.view.graph, |graph| graph.ir);

        let Some(inner_types) = executor.demand_many(&self.inner_types) else {
            let suspend_on_types = self
                .head
                .params
                .required
                .iter()
                .map(|param| &param.ty)
                .chain(std::iter::once(&self.head.return_type));

            return suspend_many!(
                self.inner_types,
                suspend_on_types
                    .map(|ty| executor.request(LowerType::new(self.view, &self.compiler, ty.0)))
                    .collect(),
                ctx
            );
        };

        let mut inner_types = inner_types.into_iter();

        let params = ctx.alloc_slice_fill_iter(
            self.head
                .params
                .required
                .iter()
                .map(|_| inner_types.next().unwrap()),
        );

        let return_type = inner_types.next().unwrap();

        // TODO: Here is where we will do monomorphization (but only for the function head)...

        let ir_func_ref = ir.funcs.alloc(ir::Func {
            mangled_name: self.head.name,
            params,
            return_type: return_type,
            is_cstyle_variadic: self.head.params.is_cstyle_vararg,
            ownership: self.head.metadata.ownership,
            abide_abi: self.head.metadata.abi.is_c(),
            basicblocks: OnceLock::new(),
        });

        Ok(ir_func_ref)
    }
}
