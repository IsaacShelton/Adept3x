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

use crate::source_file_cache::SourceFileCache;
use cli::{BuildCommand, NewCommand};
use indoc::indoc;
use lexer::Lexer;
use llvm_backend::llvm_backend;
use lower::lower;
use parser::parse;
use resolve::resolve;
use std::fs::metadata;
use std::path::Path;
use std::process::exit;

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

    let project_folder = match metadata(filepath) {
        Ok(metadata) if metadata.is_dir() => {
            unimplemented!("compiling folder");
        }
        _ => {
            if !filepath.is_file() {
                eprintln!("Expected filename to be path to file");
                exit(1);
            }

            filepath.parent().unwrap()
        }
    };

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

    // println!("{:?}", Lexer::new(content.chars()).collect::<Vec<_>>());

    let ast = match parse(Lexer::new(content.chars()), &source_file_cache, key) {
        Ok(ast) => ast,
        Err(parse_error) => {
            eprintln!("{}", parse_error);
            exit(1);
        }
    };

    // println!("{:?}", ast);

    let resolved_ast = match resolve(&ast) {
        Ok(resolved_ast) => resolved_ast,
        Err(error) => {
            eprintln!("{}", error);
            exit(1);
        }
    };

    // println!("{:?}", resolved_ast);

    let ir_module = match lower(&resolved_ast) {
        Ok(ir_module) => ir_module,
        Err(error) => {
            eprintln!("{}", error);
            exit(1);
        }
    };

    // println!("{:?}", ir_module);

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
