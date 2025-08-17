mod load_file;
mod read_file;

use super::Executable;
use crate::{
    Continuation, ExecutionCtx, Executor, canonicalize_or_error,
    module_graph::{ModuleGraphRef, ModuleGraphWeb},
    repr::Compiler,
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

    #[allow(unused)]
    module_graph_web: Option<&'env ModuleGraphWeb<'env>>,

    #[allow(unused)]
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
            .get_or_insert_with(|| ctx.alloc(ModuleGraphWeb::new(Target::HOST)));

        let compiler = ctx.alloc(Compiler {
            source_files: self.source_files,
            project_root,
        });

        let runtime = web
            .upsert_module_with_initial_part(ModuleGraphRef::Runtime, single_file)
            .out_of();

        let _ = executor.request(LoadFile::new(compiler, single_file.into(), runtime, None));

        Ok(None)
    }
}
