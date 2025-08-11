mod load_file;
mod read_file;

use super::Executable;
pub use crate::repr::Compiler;
use crate::{
    Continuation, ExecutionCtx, Executor,
    execution::main::load_file::canonicalize_or_error,
    module_graph::{ComptimeKind, ModuleGraphMeta, ModuleGraphWeb},
};
use compiler::BuildOptions;
use diagnostics::ErrorDiagnostic;
pub use load_file::LoadFile;
use source_files::SourceFiles;
use std::path::Path;
use target::Target;

#[derive(Clone, Debug)]
pub struct Main<'env> {
    #[allow(unused)]
    build_options: &'env BuildOptions,
    uncanonicalized_single_file: Option<&'env Path>,
    canonicalized_single_file: Option<&'env Path>,
    module_graph_web: Option<&'env ModuleGraphWeb<'env>>,
    source_files: &'env SourceFiles,
}

impl<'env> Main<'env> {
    pub fn new(
        build_options: &'env BuildOptions,
        uncanonicalized_single_file: Option<&'env Path>,
        source_files: &'env SourceFiles,
    ) -> Self {
        Self {
            build_options,
            uncanonicalized_single_file,
            canonicalized_single_file: None,
            source_files,
            module_graph_web: None,
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
        let Some(uncanonicalized_single_file) = self.uncanonicalized_single_file else {
            return Err(ErrorDiagnostic::plain("Must specify root file").into());
        };

        // Rust does not have `try_get_or_insert`...
        if self.canonicalized_single_file.is_none() {
            self.canonicalized_single_file =
                Some(ctx.alloc(canonicalize_or_error(uncanonicalized_single_file, None)?));
        }
        let single_file = self.canonicalized_single_file.unwrap();

        let project_root = single_file.parent();

        let web = *self
            .module_graph_web
            .get_or_insert_with(|| ctx.alloc(ModuleGraphWeb::default()));

        let comptime = web.add_module_graph(
            None,
            ModuleGraphMeta {
                title: "comptime",
                is_comptime: Some(ComptimeKind::Sandbox),
                target: Target::SANDBOX,
            },
        );
        let runtime = web.add_module_graph(
            Some(comptime),
            ModuleGraphMeta {
                title: "runtime",
                is_comptime: None,
                target: Target::HOST,
            },
        );

        let compiler = ctx.alloc(Compiler {
            source_files: self.source_files,
            project_root,
        });

        // * To incorporate files, we need to add a new handle and load the new file into it.
        // * To import modules, we need to create a new module and (should) setup the module link
        // before loading the first file into the initial handle for the new module.

        let comptime_view = web.add_module_with_initial_part(comptime);
        let _ = executor.request(LoadFile::new(
            compiler,
            single_file.into(),
            comptime_view,
            None,
        ));

        let runtime_view = web.add_module_with_initial_part(runtime);
        let _ = executor.request(LoadFile::new(
            compiler,
            single_file.into(),
            runtime_view,
            None,
        ));

        Ok(None)
    }
}
