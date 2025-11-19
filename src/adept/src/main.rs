use std::{iter::Peekable, process::ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1).peekable();

    match args.peek().map(String::as_str) {
        Some("-h" | "--help") | None => show_help(),
        Some("--server") => start_server(),
        Some("--oneshot") => start_oneshot(args),
        _ => {
            eprintln!("Please specify whether server or oneshot!");
            ExitCode::FAILURE
        }
    }
}

fn show_help() -> ExitCode {
    println!("usage: adept FILENAME");
    ExitCode::FAILURE
}

fn start_server() -> ExitCode {
    todo!()
}

fn start_oneshot(_args: Peekable<impl Iterator<Item = String>>) -> ExitCode {
    todo!()
}
