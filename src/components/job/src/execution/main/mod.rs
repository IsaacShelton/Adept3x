use super::Executable;
use crate::{
    Continuation, ExecutionCtx, Executor,
    io::{IoRequest, IoRequestHandle},
    module_graph::ModuleGraph,
    repr::{DeclHead, ValueLikeRef},
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

    io_handle: Option<IoRequestHandle>,
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
            io_handle: None,
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

        let Some(io_handle) = self.io_handle else {
            println!("Root file is {:?}", single_file);

            self.io_handle =
                Some(executor.request_io(IoRequest::ReadFile(single_file.into()), ctx.self_task()));

            return Err(Continuation::PendingIo(self.into()));
        };

        let io_response = executor
            .completed_io
            .lock()
            .unwrap()
            .get_mut(&io_handle)
            .map(|fulfilled| std::mem::take(fulfilled))
            .unwrap()
            .unwrap_fulfilled();

        println!("{:?}", io_response);

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
