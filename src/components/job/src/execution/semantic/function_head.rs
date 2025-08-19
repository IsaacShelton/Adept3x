use crate::{
    Continuation, Executable, ExecutionCtx, Executor, FuncSearch,
    module_graph::ModuleView,
    repr::{Compiler, DeclHead, ValueLikeRef},
};
use attributes::Privacy;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ResolveFunctionHead<'env> {
    view: ModuleView<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    compiler: &'env Compiler<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    head: &'env ast::FuncHead,
}

impl<'env> ResolveFunctionHead<'env> {
    pub fn new(
        view: ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        head: &'env ast::FuncHead,
    ) -> Self {
        Self {
            view,
            compiler,
            head,
        }
    }
}

impl<'env> Executable<'env> for ResolveFunctionHead<'env> {
    type Output = ();

    fn execute(
        self,
        executor: &Executor<'env>,
        _ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        self.view.add_symbol(
            Privacy::Public,
            "my_testing_function",
            DeclHead::ValueLike(ValueLikeRef::Dummy),
        );

        let found = match self.view.find_symbol(
            executor,
            FuncSearch {
                name: "my_testing_function",
            },
        ) {
            Ok(found) => found,
            Err(into_continuation) => return Err(into_continuation(self.into())),
        };

        todo!("resolve function head - found - {:?}", found)
    }
}
