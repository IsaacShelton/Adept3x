use crate::{
    Continuation, Executable, ExecutionCtx, Executor,
    module_graph::{ModuleView, ResolvedLinksetEntry},
    repr::Compiler,
};
use by_address::ByAddress;
use derivative::Derivative;
use itertools::Itertools;

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq, Hash)]
pub struct ProcessLinkset<'env> {
    view: &'env ModuleView<'env>,

    #[derivative(Debug = "ignore")]
    compiler: ByAddress<&'env Compiler<'env>>,

    linkset: ByAddress<&'env ast::Linkset>,
}

impl<'env> ProcessLinkset<'env> {
    pub fn new(
        view: &'env ModuleView<'env>,
        compiler: &'env Compiler<'env>,
        linkset: &'env ast::Linkset,
    ) -> Self {
        Self {
            view,
            compiler: ByAddress(compiler),
            linkset: ByAddress(linkset),
        }
    }
}

impl<'env> Executable<'env> for ProcessLinkset<'env> {
    type Output = ();

    fn execute(
        self,
        _executor: &Executor<'env>,
        _ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let entries = self
            .linkset
            .entries
            .iter()
            .map(|entry| match entry {
                ast::LinksetEntry::File(path) => ResolvedLinksetEntry::File(path),
                ast::LinksetEntry::Library(library) => ResolvedLinksetEntry::Library(library),
                ast::LinksetEntry::Framework(framework) => {
                    ResolvedLinksetEntry::Framework(framework)
                }
            })
            .collect_vec();

        self.view.graph(|graph| {
            graph.linksets.push(entries);
        });

        Ok(())
    }
}
