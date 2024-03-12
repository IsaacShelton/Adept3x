#![allow(dead_code, unused, unused_mut)]

mod ast;
mod error;
mod ir;
mod lexer;
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
use ast::Ast;
use lexer::Lexer;
use llvm_backend::llvm_backend;
use lower::lower;
use parser::parse;
use resolve::resolve;
use std::fs::File;
use std::io::BufReader;
use std::process::exit;
use std::string::ParseError;

fn main() {
    if std::env::args().len() != 2 {
        println!("usage: adept FILENAME");
        exit(0);
    }

    let filename = std::env::args().nth(1).unwrap();

    let source_file_cache = SourceFileCache::new();

    let key = match source_file_cache.add(&filename) {
        Ok(key) => key,
        Err(_) => {
            eprintln!("Failed to open file {}", filename);
            exit(1);
        }
    };

    let content = source_file_cache.get(key).content();

    /*
    for token in Lexer::new(reader.chars().map(|c| c.expect("valid utf8"))) {
        println!("{:?}", token);
    }
    */

    let ast = match parse(Lexer::new(content.chars()), &source_file_cache, key) {
        Ok(ast) => ast,
        Err(parse_error) => {
            eprintln!("{}", parse_error);
            exit(1);
        }
    };

    println!("{:?}", ast);

    let resolved_ast = match resolve(&ast) {
        Ok(resolved_ast) => resolved_ast,
        Err(error) => {
            eprintln!("{}", error);
            exit(1);
        }
    };

    println!("{:?}", resolved_ast);

    let ir_module = match lower(&resolved_ast) {
        Ok(ir_module) => ir_module,
        Err(error) => {
            eprintln!("{}", error);
            exit(1);
        }
    };

    println!("{:?}", ir_module);

    match unsafe { llvm_backend(&ir_module) } {
        Err(error) => eprintln!("{}", error),
        Ok(()) => (),
    }
}

