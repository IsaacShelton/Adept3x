mod client;
mod connection;
mod daemon;
mod logger;
mod queue;
mod show;

#[cfg(target_family = "unix")]
use crate::client::handle_client;
pub use crate::{connection::Connection, daemon::Daemon};
pub use queue::*;
#[cfg(target_family = "unix")]
use std::io::ErrorKind;
#[cfg(target_family = "unix")]
use std::time::Duration;
use std::{io, sync::Arc};

pub fn main_loop(daemon: Daemon) -> io::Result<()> {
    let daemon = Arc::new(daemon);

    if logger::setup().is_err() {
        eprintln!("Failed to create daemon log file");
    }

    #[cfg(target_family = "unix")]
    {
        daemon
            .listener
            .set_nonblocking(true)
            .expect("Failed to set to non-blocking");

        // Executor thread
        std::thread::spawn(|| {});

        // Accept clients
        loop {
            match daemon.listener.accept() {
                Ok((stream, address)) => {
                    let daemon = Arc::clone(&daemon);
                    std::thread::spawn(move || {
                        if daemon.idle_tracker.add_connection().is_ok() {
                            handle_client(daemon.as_ref(), stream, address);
                            daemon.idle_tracker.remove_connection();
                        }
                    });
                }
                Err(error) => {
                    if !matches!(error.kind(), ErrorKind::WouldBlock) {
                        log::error!("Failed to accept client: {:?}", error);
                    }
                }
            }

            if daemon.should_exit() {
                return Ok(());
            }

            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[cfg(target_family = "windows")]
    {
        let _ = daemon;
        panic!("Windows is not supported")
    }
}
