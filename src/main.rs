#![allow(dead_code)]

mod ast;
mod c;
mod cli;
mod error;
mod ir;
mod lexer;
mod lexical_utils;
mod line_column;
mod llvm_backend;
mod look_ahead;
mod lower;
mod parser;
mod repeating_last;
mod resolve;
mod resolved;
mod source_file_cache;
mod token;

use crate::c::preprocessor::preprocess;
use crate::source_file_cache::SourceFileCache;
use cli::{BuildCommand, NewCommand};
use indoc::indoc;
use lexer::Lexer;
use llvm_backend::llvm_backend;
use lower::lower;
use parser::{parse, parse_into};
use resolve::resolve;
use std::fmt::Display;
use std::path::Path;
use std::process::exit;
use std::{ffi::OsStr, fs::metadata};
use walkdir::{DirEntry, WalkDir};

fn main() {
    let args = match cli::Command::parse_env_args() {
        Ok(args) => args,
        Err(()) => exit(1),
    };

    match args.kind {
        cli::CommandKind::Build(build_command) => build_project(build_command),
        cli::CommandKind::New(new_command) => new_project(new_command),
    };
}

fn build_project(build_command: BuildCommand) {
    let source_file_cache = SourceFileCache::new();
    let filename = build_command.filename;
    let filepath = Path::new(&filename);

    match metadata(filepath) {
        Ok(metadata) if metadata.is_dir() => {
            compile_project(&source_file_cache, filepath);
        }
        _ => {
            if !filepath.is_file() {
                eprintln!("Expected filename to be path to file");
                exit(1);
            }

            if filepath.extension().unwrap_or_default() == "h" {
                let content = std::fs::read_to_string(filepath).expect("file to exist");
                println!("{:?}", preprocess(&content).unwrap());
                return;
            }

            let project_folder = filepath.parent().unwrap();
            compile(&source_file_cache, project_folder, &filename);
        }
    }
}

fn exit_unless<T, E: Display>(result: Result<T, E>) -> T {
    match result {
        Ok(value) => value,
        Err(err) => {
            eprintln!("{}", err);
            exit(1);
        }
    }
}

fn compile_project(source_file_cache: &SourceFileCache, filepath: &Path) {
    let folder_path = filepath;
    let walker = WalkDir::new(filepath).min_depth(1).into_iter();
    let mut ast = None;

    let output_binary_filepath = folder_path.join("a.out");
    let output_object_filepath = folder_path.join("a.o");

    for entry in walker.filter_entry(|e| !is_hidden(e)) {
        let entry = entry.expect("walk dir");
        let extension = entry.path().extension();

        let is_header = match extension.and_then(OsStr::to_str) {
            Some("adept") => false,
            Some("h") => true,
            _ => continue,
        };

        let filename = entry.path().to_string_lossy().to_string();

        let key = match source_file_cache.add(&filename) {
            Ok(key) => key,
            Err(_) => {
                eprintln!("Failed to open file {}", filename);
                exit(1);
            }
        };

        let content = source_file_cache.get(key).content();

        if !is_header {
            let lexer = Lexer::new(content.chars());

            if let Some(ast) = &mut ast {
                exit_unless(parse_into(
                    lexer,
                    &source_file_cache,
                    key,
                    ast,
                    filename.to_string(),
                ));
            } else {
                exit_unless(parse(lexer, &source_file_cache, key));
            }
        } else {
            let mut preprocessed = exit_unless(c::preprocessor::preprocess(content));
            let lexer = c::Lexer::new(preprocessed.drain(..));

            if let Some(ast) = &mut ast {
                exit_unless(c::parse_into(
                    lexer,
                    &source_file_cache,
                    key,
                    ast,
                    filename.to_string(),
                ));
            } else {
                exit_unless(c::parse(lexer, &source_file_cache, key));
            }
        }
    }

    let ast = if let Some(ast) = ast {
        ast
    } else {
        eprintln!("must have at least one adept file in directory in order to compile");
        exit(1);
    };

    let resolved_ast = exit_unless(resolve(&ast));
    let ir_module = exit_unless(lower(&resolved_ast));

    exit_unless(unsafe {
        llvm_backend(&ir_module, &output_object_filepath, &output_binary_filepath)
    });
}

fn compile(source_file_cache: &SourceFileCache, project_folder: &Path, filename: &str) {
    let output_binary_filepath = project_folder.join("a.out");
    let output_object_filepath = project_folder.join("a.o");

    let key = match source_file_cache.add(&filename) {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Failed to open file {}", filename);
            exit(1);
        }
    };

    let content = source_file_cache.get(key).content();

    let ast = match parse(Lexer::new(content.chars()), &source_file_cache, key) {
        Ok(ast) => ast,
        Err(parse_error) => {
            eprintln!("{}", parse_error);
            exit(1);
        }
    };

    let resolved_ast = match resolve(&ast) {
        Ok(resolved_ast) => resolved_ast,
        Err(error) => {
            eprintln!("{}", error);
            exit(1);
        }
    };

    let ir_module = match lower(&resolved_ast) {
        Ok(ir_module) => ir_module,
        Err(error) => {
            eprintln!("{}", error);
            exit(1);
        }
    };

    match unsafe { llvm_backend(&ir_module, &output_object_filepath, &output_binary_filepath) } {
        Err(error) => eprintln!("{}", error),
        Ok(()) => (),
    }
}

fn new_project(new_command: NewCommand) {
    if let Err(_) = std::fs::create_dir(&new_command.project_name) {
        eprintln!(
            "Failed to create project directory '{}'",
            &new_command.project_name
        );
        exit(1);
    }

    let imports = indoc! {r#"
        import std::prelude
    "#};

    let main = indoc! {r#"

        func main {
            println("Hello World!")
        }
    "#};

    let lock = indoc! {r#"
        std::prelude 1.0 731f4cbc9ba52451245d8f67961b640111e522972a6a4eff97c88f7ff07b0b59
    "#};

    put_file(&new_command.project_name, "_.imports", imports);
    put_file(&new_command.project_name, "_.lock", lock);
    put_file(&new_command.project_name, "main.adept", main);
    println!("Project created!");
}

fn put_file(directory_name: &str, filename: &str, content: &str) {
    let path = std::path::Path::new(directory_name).join(filename);

    if let Err(_) = std::fs::write(&path, content) {
        eprintln!("Failed to create {} file", filename);
        exit(1);
    }
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with("."))
        .unwrap_or(false)
}
