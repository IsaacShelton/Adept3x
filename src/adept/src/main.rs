use daemon::daemonize_main;
use std::{iter::Peekable, process::ExitCode, time::Duration};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1).peekable();

    match args.peek().map(String::as_str) {
        Some("-h" | "--help") | None => show_help(),
        Some("--server") => start_server(),
        Some("--oneshot") => start_oneshot(args),
        _ => start_server(),
    }
}

fn show_help() -> ExitCode {
    println!("usage: adept FILENAME");
    ExitCode::FAILURE
}

fn start_server() -> ExitCode {
    let path = std::env::current_dir().unwrap();
    let max_idle_time = Duration::from_secs(5 * 60);
    daemonize_main(path, max_idle_time)
}

fn start_oneshot(_args: Peekable<impl Iterator<Item = String>>) -> ExitCode {
    todo!()
}
