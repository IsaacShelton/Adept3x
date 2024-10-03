use super::PragmaSection;
use crate::{
    ast::{AstWorkspace, Settings},
    cli::BuildOptions,
    compiler::Compiler,
    interpreter_env::{run_build_system_interpreter, setup_build_system_interpreter_symbols},
    lower::lower,
    parser::error::ParseErrorKind,
    resolve::resolve,
    show::{into_show, Show},
    target::Target,
    workspace::fs::Fs,
};
use indexmap::IndexMap;
use std::path::Path;

impl PragmaSection {
    pub fn run(
        mut self,
        base_compiler: &Compiler,
        path: &Path,
        existing_settings: Option<Settings>,
    ) -> Result<Settings, Box<dyn Show>> {
        let compiler = Compiler {
            options: BuildOptions {
                emit_llvm_ir: false,
                emit_ir: false,
                interpret: true,
                coerce_main_signature: false,
                excute_result: false,
                use_pic: None,
                allow_experimental_pragma_features: false,
                target: Target::default(),
                infrastructure: None,
            },
            target: base_compiler.target,
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
            lower(&compiler.options, &resolved_ast, &compiler.target).map_err(into_show)?;

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

        let Some(adept_version) = user_settings
            .version
            .or_else(|| existing_settings.map(|settings| settings.adept_version))
        else {
            return Err(into_show(
                ParseErrorKind::Other {
                    message: "No Adept version was specifed for module! Use `pragma => adept(\"3.0\")` at the top of the module file".into(),
                }
                .at(self.pragma_source),
            ));
        };

        if *base_compiler.version.get_or_init(|| adept_version.clone()) != adept_version {
            return Err(into_show(
                ParseErrorKind::Other {
                    message: "Using multiple Adept versions at the same time is not support yet"
                        .into(),
                }
                .at(self.pragma_source),
            ));
        }

        // Update linking information
        for link_filename in user_settings.link_filenames.drain() {
            let link_filename = path
                .parent()
                .expect("file has parent")
                .join(link_filename)
                .to_str()
                .expect("valid utf-8 filename")
                .to_owned();

            base_compiler
                .link_filenames
                .map_insert(link_filename, |_| (), |_, _| ());
        }

        for link_framework in user_settings.link_frameworks.drain() {
            base_compiler
                .link_frameworks
                .map_insert(link_framework, |_| (), |_, _| ());
        }

        Ok(Settings {
            adept_version,
            debug_skip_merging_helper_exprs: user_settings.debug_skip_merging_helper_exprs,
            imported_namespaces: user_settings.imported_namespaces,
            assume_int_at_least_32_bits: user_settings.assume_int_at_least_32_bits,
            imported_folders: user_settings.imported_folders,
            folders_as_namespaces: user_settings.folders_as_namespaces,
        })
    }
}
