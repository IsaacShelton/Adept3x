use super::{BuildCommand, supported_targets::warn_if_unsupported_target};
use crate::Invoke;
use build_pp_ast::preprocess;
use build_workspace::{compile_single_file_only, compile_workspace};
use compiler::Compiler;
use diagnostics::{DiagnosticFlags, Diagnostics, unerror};
use source_files::SourceFiles;
use std::{fs::metadata, path::Path};
use text::{TextPeeker, TextStreamer};

impl Invoke for BuildCommand {
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

    let header_contents = TextPeeker::new(TextStreamer::new(
        source_files.get(header_key).content().chars(),
        header_key,
    ));

    let preprocessed = unerror(
        preprocess(header_contents, &compiler.diagnostics),
        &source_files,
    )?;

    println!("{preprocessed:?}");
    return Ok(());
}
