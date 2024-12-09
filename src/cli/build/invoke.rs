use super::{supported_targets::warn_if_unsupported_target, BuildCommand};
use crate::{
    c,
    cli::CliInvoke,
    compiler::Compiler,
    diagnostics::{DiagnosticFlags, Diagnostics},
    single_file_only::compile_single_file_only,
    source_files::SourceFiles,
    text::{IntoText, IntoTextStream},
    unerror::unerror,
    workspace::compile_workspace,
};
use std::{fs::metadata, path::Path};

impl CliInvoke for BuildCommand {
    fn invoke(self) -> Result<(), ()> {
        let BuildCommand { filename, options } = self;
        let source_files = SourceFiles::new();
        let filepath = Path::new(&filename);
        let diagnostics = Diagnostics::new(&source_files, DiagnosticFlags::default());
        let target = options.target;

        let Ok(metadata) = metadata(filepath) else {
            eprintln!("error: File or folder does not exist");
            return Err(());
        };

        warn_if_unsupported_target(&target, &diagnostics);

        let mut compiler = Compiler {
            options,
            source_files: &source_files,
            diagnostics: &diagnostics,
            version: Default::default(),
            link_filenames: Default::default(),
            link_frameworks: Default::default(),
        };

        if metadata.is_dir() {
            compile_workspace(&mut compiler, filepath, None)
        } else if filepath.extension().unwrap_or_default() == "h" {
            compile_header(&compiler, filepath)
        } else {
            compile_single_file_only(&mut compiler, filepath.parent().unwrap(), filepath)
        }
    }
}

fn compile_header(compiler: &Compiler, filepath: &Path) -> Result<(), ()> {
    let source_files = compiler.source_files;

    let content = std::fs::read_to_string(filepath).map_err(|err| {
        eprintln!("{}", err);
        ()
    })?;

    let header_key = source_files.add(filepath.into(), content);

    let header_contents = source_files
        .get(header_key)
        .content()
        .chars()
        .into_text_stream(header_key)
        .into_text();

    let preprocessed = unerror(
        c::preprocessor::preprocess(header_contents, &compiler.diagnostics),
        &source_files,
    )?;

    println!("{preprocessed:?}");
    return Ok(());
}
