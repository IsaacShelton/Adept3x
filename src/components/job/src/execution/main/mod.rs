mod read_file;

use super::Executable;
use crate::{
    Continuation, ExecutionCtx, Executor,
    execution::main::read_file::ReadFile,
    module_graph::ModuleGraph,
    repr::{DeclHead, ValueLikeRef},
    sub_task::SubTask,
};
use attributes::Privacy;
use compiler::BuildOptions;
use diagnostics::ErrorDiagnostic;
use std::path::Path;

#[derive(Clone, Debug)]
pub struct Main<'env> {
    #[allow(unused)]
    build_options: &'env BuildOptions,

    #[allow(unused)]
    project_folder: &'env Path,

    #[allow(unused)]
    single_file: Option<&'env Path>,

    #[allow(unused)]
    module_graph: Option<&'env ModuleGraph<'env>>,

    read_file: Option<ReadFile>,
    read_file2: Option<ReadFile>,
}

impl<'env> Main<'env> {
    pub fn new(
        build_options: &'env BuildOptions,
        project_folder: &'env Path,
        single_file: Option<&'env Path>,
    ) -> Self {
        Self {
            build_options,
            project_folder,
            single_file,
            module_graph: None,
            read_file: None,
            read_file2: None,
        }
    }
}

impl<'env> Executable<'env> for Main<'env> {
    // The filepath to execute when completed
    type Output = Option<&'env Path>;

    fn execute(
        mut self,
        executor: &Executor<'env>,
        ctx: &mut ExecutionCtx<'env>,
    ) -> Result<Self::Output, Continuation<'env>> {
        let Some(single_file) = self.single_file else {
            return Err(ErrorDiagnostic::plain("Must specify root file").into());
        };

        let result1 = execute_sub_task!(
            self,
            self.read_file
                .get_or_insert_with(|| ReadFile::new(single_file.into())),
            executor,
            ctx
        );

        let result2 = execute_sub_task!(
            self,
            self.read_file2
                .get_or_insert_with(|| ReadFile::new("bacon.toml".into())),
            executor,
            ctx
        );

        println!("{:?}", result1);
        println!("{:?}", result2);

        let module_graph = *self
            .module_graph
            .get_or_insert_with(|| ctx.alloc(ModuleGraph::default()));

        let handle = module_graph.add_module_with_initial_part();

        module_graph.add_symbol(
            Privacy::Public,
            "test",
            DeclHead::ValueLike(ValueLikeRef::Dummy),
            handle,
        );

        module_graph.add_symbol(
            Privacy::Private,
            "test",
            DeclHead::ValueLike(ValueLikeRef::Dummy),
            handle,
        );

        let incorporated = module_graph.add_part(handle);

        module_graph.add_symbol(
            Privacy::Protected,
            "hello",
            DeclHead::ValueLike(ValueLikeRef::Dummy),
            incorporated,
        );

        Ok(None)
    }
}
