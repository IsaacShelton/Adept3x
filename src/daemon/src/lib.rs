mod error;

pub use error::*;
use smol::{Timer, net::TcpStream};
use std::{
    fs::remove_file,
    io,
    process::{Command, ExitCode},
    time::Duration,
};

pub fn start() -> ExitCode {
    match try_become() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", error);
            ExitCode::FAILURE
        }
    }
}

/// Become the daemon process
pub fn try_become() -> io::Result<()> {
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let filepath = cwd.join("adeptd.lock");

    let Some(lock) = lock_file::acquire(&filepath)? else {
        eprintln!("Daemon already running.");
        return Ok(());
    };

    eprintln!("Starting daemon...");
    daemon_scheduler::main()?;

    eprintln!("Daemon shutting down...");
    drop(lock);

    remove_file(&filepath)
}

/// Tries to connect to the daemon process. If the daemon process
/// is not running yet, this function attempts to launch it.
pub async fn connect() -> Result<TcpStream, StartError> {
    if let Ok(connection) = TcpStream::connect("127.0.0.1:6000").await {
        eprintln!("Connected to existing daemon.");
        return Ok(connection);
    }

    spawn()?;

    for _ in 0..10 {
        if let Ok(connection) = TcpStream::connect("127.0.0.1:6000").await {
            return Ok(connection);
        }
        Timer::after(Duration::from_millis(20)).await;
    }

    Err(StartError::FailedToStart)
}

pub fn spawn() -> std::io::Result<()> {
    let exe = std::env::current_exe()?;

    // WARNING: SECURITY: This could lead to privilege escalation
    // to the level the compiler is running at if an attacker
    // overwrites the current executable.
    // TL;DR - Don't let the compiler executable be changed
    // by less privileged users.
    Command::new(exe).arg("--daemon").spawn()?;
    Ok(())
}
