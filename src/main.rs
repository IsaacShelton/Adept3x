
mod lexer;
mod look_ahead;

use std::io::BufReader;
use std::process::exit;
use std::fs::File;
use utf8_chars::BufReadCharsExt;
use lexer::Lexer;

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

    for token in Lexer::new(reader.chars().map(|c| c.expect("valid utf8"))) {
        println!("{:?}", token);
    }
}
