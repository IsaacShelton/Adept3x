mod error;

use daemon::{Connection, Daemon};
pub use error::*;
#[cfg(target_family = "unix")]
use std::os::unix::net::{UnixListener, UnixStream};
#[cfg(target_family = "unix")]
use std::time::Duration;
use std::{
    fs::remove_file,
    io,
    path::Path,
    process::{Command, ExitCode},
};

pub fn start() -> ExitCode {
    match try_become() {
        Ok(_) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{}", error);
            ExitCode::FAILURE
        }
    }
}

pub fn try_become() -> io::Result<()> {
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    try_become_impl(&cwd.join("adeptd.lock"))
}

#[cfg(target_family = "unix")]
pub fn try_become_impl(filepath: &Path) -> io::Result<()> {
    let listener = loop {
        // Attempt to become the server for Unix Domain Socket
        // at the specified filepath.
        match UnixListener::bind(&filepath) {
            Ok(listener) => {
                // If we acquired access to be the server for this
                // Unix Domain Socket, then we are now the daemon.
                break listener;
            }
            Err(error) => {
                // Otherwise if the address is already "in-use",
                // we should check to see if there is a stale
                // Unix Domain Socket file that already exists
                // that we can delete and try again.
                if let io::ErrorKind::AddrInUse = error.kind() {
                    // Try to connect to the supposedly "in-use"
                    // Unix Domain Socket.
                    if UnixStream::connect(&filepath).is_err() {
                        // If we failed, then it's likely that
                        // we're using a stale Unix Domain Socket file.
                        // Try to delete it, and if successful,
                        // then we can retry becoming the server.
                        if remove_file(&filepath).is_ok() {
                            continue;
                        }
                    }

                    // Otherwise, an instance of the daemon is
                    // very likely already running here, so we
                    // shouldn't try to take its place.
                    eprintln!("Daemon already running.");
                    return Ok(());
                }

                // Any error that's not an "address in-use" error
                // we can't recover from.
                return Err(error);
            }
        }
    };

    eprintln!("Got listener {:?}", listener);

    let result = daemon::main_loop(Daemon::new(listener));
    let _ = remove_file(&filepath);
    result
}

/// Tries to connect to the daemon process. If the daemon process
/// is not running yet, then this function attempts to launch it.
#[cfg(target_family = "unix")]
pub fn connect() -> Result<Connection, StartError> {
    let cwd = std::env::current_dir().expect("Failed to get current directory");
    let filepath = cwd.join("adeptd.lock");

    // 1) Check if we can connect to Unix Domain Socket
    if let Ok(stream) = UnixStream::connect(&filepath) {
        // 2) If okay, then this client has established a connection.
        eprintln!("Connected to existing daemon.");
        return Ok(Connection {
            stream: stream.into(),
        });
    }

    // 3) If failed, spawn daemon.
    spawn()?;

    // 4) Try to connect again a few times.
    for _ in 0..10 {
        if let Ok(stream) = UnixStream::connect(&filepath) {
            eprintln!("Connected to spawned daemon.");
            return Ok(Connection {
                stream: stream.into(),
            });
        }

        std::thread::sleep(Duration::from_millis(20));
    }

    // 5) If still can't connect, then the daemon likely couldn't start.
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
