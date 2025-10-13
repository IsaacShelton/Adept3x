use crate::{
    Continuation, Executable, ExecutionCtx, Executor,
    module_graph::ModuleView,
    repr::{Compiler, DeclHead, DeclHeadTypeLike, TypeHead, TypeHeadRest, TypeHeadRestKind},
};
use by_address::ByAddress;
use derivative::Derivative;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ProcessStructure<'env> {
    view: &'env ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    structure: ByAddress<&'env ast::Struct>,

    #[derivative(Hash = "ignore")]
    #[derivative(Debug = "ignore")]
    #[derivative(PartialEq = "ignore")]
    resolved_head: bool,
}

impl<'env> ProcessStructure<'env> {
    pub fn new(
        view: &'env ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        structure: &'env ast::Struct,
    ) -> Self {
        Self {
            view,
            compiler: ByAddress(compiler),
            structure: ByAddress(structure),
            resolved_head: false,
        }
    }
}

impl<'env> Executable<'env> for ProcessStructure<'env> {
    type Output = ();

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        // Resolve struct head
        if !self.resolved_head {
            let type_head = ctx.alloc(TypeHead {
                name: &self.structure.name,
                arity: 0,
                rest: TypeHeadRest {
                    kind: TypeHeadRestKind::Struct(self.structure),
                    view: self.view,
                },
            });

            self.view.add_symbol(
                self.structure.privacy,
                &self.structure.name,
                DeclHead::TypeLike(DeclHeadTypeLike::Type(type_head)),
            );

            executor.wake_pending_search(self.view.graph, &self.structure.name);
            self.resolved_head = true;
        }

        // Resolve struct body

        Ok(())
    }
}
