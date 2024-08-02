#![allow(dead_code)]
#![allow(clippy::diverging_sub_expression)]
#![allow(clippy::module_name_repetitions)]
#![feature(string_remove_matches)]
#![feature(never_type)]
#![feature(exhaustive_patterns)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(once_cell_try_insert)]

mod ast;
mod borrow;
mod c;
mod cli;
mod compiler;
mod data_units;
mod diagnostics;
mod file_id;
mod generate_workspace;
mod inflow;
mod interpreter;
mod interpreter_env;
mod ir;
mod lexer;
mod lexical_utils;
mod line_column;
mod llvm_backend;
mod look_ahead;
mod lower;
mod parser;
mod pragma_section;
mod repeating_last;
mod resolve;
mod resolved;
mod show;
mod single_file_only;
mod source_files;
mod tag;
mod target_info;
mod text;
mod token;
mod try_insert_index_map;
mod version;
mod workspace;

use crate::{cli::BuildCommand, show::Show, source_files::SourceFiles, text::IntoText};
use compiler::Compiler;
use diagnostics::{DiagnosticFlags, Diagnostics, WarningDiagnostic};
use generate_workspace::new_project;
use single_file_only::compile_single_file_only;
use std::{fs::metadata, path::Path, process::exit};
use target_info::TargetInfo;
use workspace::compile_workspace;

fn main() {
    let Ok(args) = cli::Command::parse_env_args() else {
        exit(1)
    };

    match args.kind {
        cli::CommandKind::Build(build_command) => build_project(build_command),
        cli::CommandKind::New(new_command) => new_project(new_command),
    }
}

fn build_project(build_command: BuildCommand) {
    let BuildCommand { filename, options } = build_command;
    let source_file_cache = SourceFiles::new();
    let filepath = Path::new(&filename);
    let diagnostics = Diagnostics::new(&source_file_cache, DiagnosticFlags::default());

    let Ok(metadata) = metadata(filepath) else {
        eprintln!("error: File or folder does not exist");
        exit(1);
    };

    // TODO: Determine this based on default target triple
    let target_info = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        diagnostics.push(WarningDiagnostic::plain(
            "Using only supported platform aarch64 darwin",
        ));

        TargetInfo {
            kind: target_info::TargetInfoKind::AARCH64,
            ms_abi: false,
            is_darwin: true,
        }
    } else {
        diagnostics.push(WarningDiagnostic::plain(
            "Your platform is not supported yet, using arbitrary abi",
        ));

        TargetInfo::arbitrary()
    };

    let mut compiler = Compiler {
        options,
        target_info: &target_info,
        source_file_cache: &source_file_cache,
        diagnostics: &diagnostics,
        version: Default::default(),
        link_filenames: Default::default(),
        link_frameworks: Default::default(),
    };

    if metadata.is_dir() {
        compile_workspace(&mut compiler, filepath);
    } else {
        if filepath.extension().unwrap_or_default() == "h" {
            let source_file_cache = compiler.source_file_cache;

            let content = std::fs::read_to_string(filepath)
                .map_err(|err| {
                    eprintln!("{}", err);
                    exit(1);
                })
                .unwrap();

            let header_key = source_file_cache.add(filepath.into(), content);

            let header_contents = source_file_cache
                .get(header_key)
                .content()
                .chars()
                .into_text(header_key);

            let preprocessed = exit_unless(
                c::preprocessor::preprocess(header_contents, &diagnostics),
                &source_file_cache,
            );

            println!("{preprocessed:?}");
            return;
        }

        let project_folder = filepath.parent().unwrap();
        compile_single_file_only(&mut compiler, project_folder, &filename);
    }
}

fn exit_unless<T, E: Show>(result: Result<T, E>, source_file_cache: &SourceFiles) -> T {
    match result {
        Ok(value) => value,
        Err(err) => {
            let mut message = String::new();

            err.show(&mut message, source_file_cache)
                .expect("show error message");

            eprintln!("{message}");
            exit(1);
        }
    }
}
