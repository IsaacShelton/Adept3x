use super::PragmaSection;
use crate::{
    ast::AstWorkspace,
    cli::BuildOptions,
    compiler::Compiler,
    interpreter_env::{run_build_system_interpreter, setup_build_system_interpreter_symbols},
    lower::lower,
    parser::error::ParseErrorKind,
    resolve::resolve,
    show::{into_show, Show},
    workspace::fs::Fs,
};
use indexmap::IndexMap;
use std::path::Path;

impl PragmaSection {
    pub fn run(mut self, base_compiler: &Compiler, path: &Path) -> Result<(), Box<dyn Show>> {
        let compiler = Compiler {
            options: BuildOptions {
                emit_llvm_ir: false,
                emit_ir: false,
                interpret: true,
                coerce_main_signature: false,
            },
            target_info: base_compiler.target_info,
            source_files: base_compiler.source_files,
            diagnostics: base_compiler.diagnostics,
            version: Default::default(),
            link_filenames: Default::default(),
            link_frameworks: Default::default(),
        };

        setup_build_system_interpreter_symbols(&mut self.ast_file);

        let fs = Fs::new();
        let fs_node_id = fs.insert(path, None).expect("inserted");
        let files = IndexMap::from_iter(std::iter::once((fs_node_id, self.ast_file)));
        let workspace = AstWorkspace::new(fs, files, base_compiler.source_files, None);

        let resolved_ast = resolve(&workspace, &compiler.options).map_err(into_show)?;

        let ir_module =
            lower(&compiler.options, &resolved_ast, &compiler.target_info).map_err(into_show)?;

        let mut user_settings = run_build_system_interpreter(&resolved_ast, &ir_module)
            .map_err(|interpretter_error| {
                into_show(
                    ParseErrorKind::Other {
                        message: interpretter_error.to_string(),
                    }
                    .at(self.pragma_source),
                )
            })?
            .syscall_handler;

        // Update version
        if let Some(version) = user_settings.version {
            if base_compiler.version.try_insert(version).is_err() {
                return Err(into_show(
                    ParseErrorKind::Other {
                        message: "Adept version was already specified".into(),
                    }
                    .at(self.pragma_source),
                ));
            }
        }

        // Update linking information
        for link_filename in user_settings.link_filenames.drain() {
            base_compiler
                .link_filenames
                .map_insert(link_filename, |_| (), |_, _| ());
        }

        for link_framework in user_settings.link_frameworks.drain() {
            base_compiler
                .link_frameworks
                .map_insert(link_framework, |_| (), |_, _| ());
        }

        Ok(())
    }
}
