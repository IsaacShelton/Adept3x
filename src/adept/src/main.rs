use daemon::{connect_to_daemon, server_main, try_become_daemon};
use std::{iter::Peekable, process::ExitCode, time::Duration};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1).peekable();

    match args.peek().map(String::as_str) {
        Some("-h" | "--help") | None => show_help(),
        Some("--daemon") => start_daemon(args),
        Some("--oneshot") => start_oneshot(args),
        Some("--incremental") => start_incremental(),
        Some("--language-server") => start_language_server(args),
        _ => start_incremental(),
    }
}

fn show_help() -> ExitCode {
    println!("usage: adept FILENAME");
    ExitCode::FAILURE
}

fn start_daemon(_args: Peekable<impl Iterator<Item = String>>) -> ExitCode {
    let path = std::env::current_dir().expect("failed to get current directory");
    let max_idle_time = Duration::from_secs(5 * 60);

    match try_become_daemon(&path.clone(), || server_main(max_idle_time)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{}", err);
            ExitCode::FAILURE
        }
    }
}

fn start_incremental() -> ExitCode {
    let connection = match connect_to_daemon() {
        Ok(connection) => connection,
        Err(err) => {
            eprintln!("{}", err);
            return ExitCode::FAILURE;
        }
    };

    println!("Connected! {:?}", connection);
    ExitCode::SUCCESS
}

fn start_oneshot(_args: Peekable<impl Iterator<Item = String>>) -> ExitCode {
    todo!()
}

fn start_language_server(_args: Peekable<impl Iterator<Item = String>>) -> ExitCode {
    language_server::start()
}
