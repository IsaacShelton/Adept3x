mod load_file;
mod read_file;

use super::Executable;
pub use crate::repr::Compiler;
use crate::{Continuation, ExecutionCtx, Executor, module_graph::ModuleGraph};
use compiler::BuildOptions;
use diagnostics::ErrorDiagnostic;
pub use load_file::LoadFile;
use source_files::SourceFiles;
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

    source_files: &'env SourceFiles,
}

impl<'env> Main<'env> {
    pub fn new(
        build_options: &'env BuildOptions,
        project_folder: &'env Path,
        single_file: Option<&'env Path>,
        source_files: &'env SourceFiles,
    ) -> Self {
        Self {
            build_options,
            project_folder,
            single_file,
            source_files,
            module_graph: None,
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

        let module_graph = *self
            .module_graph
            .get_or_insert_with(|| ctx.alloc(ModuleGraph::default()));

        let compiler = ctx.alloc(Compiler {
            source_files: self.source_files,
        });

        // * To incorporate files, we need to add a new handle and load the new file into it.
        // * To import modules, we need to create a new module and (should) setup the module link
        // before loading the first file into the initial handle for the new module.

        let handle = module_graph.add_module_with_initial_part();

        let _ = executor.request(LoadFile::new(compiler, single_file.into(), handle));

        Ok(None)
    }
}
