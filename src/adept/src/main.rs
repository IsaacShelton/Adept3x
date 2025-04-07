use cli;
use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::Command::parse().and_then(cli::Invoke::invoke) {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}
