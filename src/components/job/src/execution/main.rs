use super::Executable;
use crate::{
    BuiltinTypes, Continuation, ExecutionCtx, Executor, ProcessFile, Suspend,
    build_llvm_ir::llvm_backend,
    canonicalize_or_error,
    module_graph::{ModuleGraphRef, ModuleGraphWeb},
    repr::Compiler,
};
use compiler::BuildOptions;
use diagnostics::ErrorDiagnostic;
use llvm_sys::core::LLVMIsMultithreaded;
use source_files::SourceFiles;
use std::{ffi::OsString, fs::create_dir_all, path::Path};
use target::Target;

#[derive(Clone, Debug)]
pub struct Main<'env> {
    #[allow(unused)]
    build_options: &'env BuildOptions,
    uncanonicalized_single_file: Option<&'env Path>,
    canonicalized_single_file: Option<&'env Path>,
    module_graph_web: Option<&'env ModuleGraphWeb<'env>>,
    source_files: &'env SourceFiles,
    all_done: Suspend<'env, ()>,
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
            all_done: None,
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
            self.canonicalized_single_file = Some(ctx.alloc(canonicalize_or_error(
                None,
                uncanonicalized_single_file,
                None,
                ModuleGraphRef::Runtime,
            )?));
        }
        let single_file = self.canonicalized_single_file.unwrap();
        let project_root = single_file.parent().expect("root file to have parent");

        let compiler = ctx.alloc(Compiler {
            source_files: self.source_files,
            project_root,
            builtin_types: ctx.alloc(BuiltinTypes::new(ctx)),
            runtime_target: self.build_options.target,
            link_filenames: Default::default(),
            link_frameworks: Default::default(),
        });

        let web = *self
            .module_graph_web
            .get_or_insert_with(|| ctx.alloc(ModuleGraphWeb::new(Target::HOST, ctx)));

        let meta = web.graph(ModuleGraphRef::Runtime, |graph| graph.meta());

        let runtime = web
            .upsert_module_with_initial_part(ModuleGraphRef::Runtime, meta, single_file)
            .out_of();

        // Ensure multi-threading support is enabled for the version of LLVM being used
        if unsafe { LLVMIsMultithreaded() } == 0 {
            return Err(ErrorDiagnostic::plain(
                "Your LLVM version does not support multi-threading! Please upgrade.",
            )
            .into());
        }

        let Some(_) = self.all_done else {
            return suspend!(
                self.all_done,
                executor.spawn(ProcessFile::new(
                    compiler,
                    single_file.into(),
                    runtime,
                    None
                )),
                ctx
            );
        };

        let ir = runtime.web.graph(runtime.graph, |graph| graph.ir);
        let meta = runtime
            .web
            .graph(runtime.graph, |graph| graph.meta().clone());

        let binary_artifacts_folder = compiler.project_root.join("bin");
        let object_files_folder = compiler.project_root.join("obj");
        create_dir_all(&binary_artifacts_folder).expect("failed to create bin folder");
        create_dir_all(&object_files_folder).expect("failed to create obj folder");
        let target = &meta.target;
        let project_name = project_name(compiler.project_root);

        let object_file_filepath =
            object_files_folder.join(target.default_object_file_name(&project_name));

        let executable_filepath =
            binary_artifacts_folder.join(target.default_executable_name(&project_name));

        let linking_duration = unsafe {
            llvm_backend(
                ctx,
                compiler,
                self.build_options,
                ir,
                &meta,
                &object_file_filepath,
                &executable_filepath,
                executor.diagnostics,
            )?
        };

        println!("Linked in {}ms", linking_duration.as_millis());
        Ok(None)
    }
}

fn project_name(project_folder: &Path) -> OsString {
    project_folder
        .file_name()
        .map(OsString::from)
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .and_then(|dir| dir.file_name().map(OsString::from))
        })
        .unwrap_or_else(|| OsString::from("main"))
}
