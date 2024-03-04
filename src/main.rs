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
mod resolve;
mod resolved;
mod token;

use lexer::Lexer;
use llvm_backend::llvm_backend;
use lower::lower;
use parser::parse;
use resolve::resolve;
use std::fs::File;
use std::io::BufReader;
use std::process::exit;
use utf8_chars::BufReadCharsExt;

fn main() {
    if std::env::args().len() != 2 {
        println!("usage: adept FILENAME");
        exit(0);
    }

    let filename = std::env::args().nth(1).unwrap();

    if filename.ends_with(".h") {
        use lang_c::driver::{parse_preprocessed, Config};
        let config = Config::default();
        println!(
            "{:?}",
            parse_preprocessed(
                &config,
                std::fs::read_to_string(filename).expect("Failed to read file")
            )
        );
        return;
    }

    let file = match File::open(&filename) {
        Ok(file) => file,
        Err(_) => {
            eprintln!("Failed to open file {}", filename);
            exit(1);
        }
    };

    let mut reader = BufReader::new(file);

    /*
    for token in Lexer::new(reader.chars().map(|c| c.expect("valid utf8"))) {
        println!("{:?}", token);
    }
    */

    let ast = match parse(
        Lexer::new(reader.chars().map(|c| c.expect("valid utf8"))),
        filename.clone(),
    ) {
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
