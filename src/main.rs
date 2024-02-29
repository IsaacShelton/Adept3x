mod ast;
mod lexer;
mod line_column;
mod look_ahead;
mod parser;
mod token;

use colored::Colorize;
use lexer::Lexer;
use parser::parse;
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

    let ast = parse(Lexer::new(reader.chars().map(|c| c.expect("valid utf8"))));
    match ast {
        Ok(ast) => println!("{:?}", ast),
        Err(parse_error) => {
            eprintln!(
                "{}:{}:{}: {}: {}",
                filename,
                parse_error.location.line,
                parse_error.location.column,
                "error".bright_red().bold(),
                parse_error.message
            );
        }
    };
}
