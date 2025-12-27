use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1).peekable();

    match args.peek().map(String::as_str) {
        Some("-h" | "--help") | None => show_help(),
        Some("--daemon") => daemon_init::start(),
        Some("--language-server") => language_server::start(),
        _ => show_help(),
    }
}

fn show_help() -> ExitCode {
    println!("usage: adept FILENAME");
    ExitCode::FAILURE
}
