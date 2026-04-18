mod connection;
mod daemon;
mod handle_client;
mod logger;
mod queue;
mod show;

pub use crate::{connection::Connection, daemon::Daemon};
pub use queue::*;
use std::{io, sync::Arc};

pub fn main_loop(daemon: Daemon) -> io::Result<()> {
    let daemon = Arc::new(daemon);

    if logger::setup().is_err() {
        eprintln!("Failed to create daemon log file");
    }

    #[cfg(target_family = "unix")]
    {
        use crate::handle_client::handle_client;
        use std::{io::ErrorKind, time::Duration};

        daemon
            .listener
            .set_nonblocking(true)
            .expect("Failed to set to non-blocking");

        // Executor thread
        std::thread::spawn(|| {});

        // Accept clients
        loop {
            match daemon.listener.accept() {
                Ok((stream, _address)) => {
                    let daemon = Arc::clone(&daemon);
                    std::thread::spawn(move || {
                        if daemon.idle_tracker.add_connection().is_ok() {
                            stream.set_nonblocking(false).unwrap();
                            if let Err(err) =
                                stream.set_read_timeout(Some(Duration::from_millis(50)))
                            {
                                log::error!(
                                    "Client connection closed before able to set timeout - {}",
                                    err
                                );
                                return;
                            }

                            let desc = format!("{:?}", stream.local_addr());
                            handle_client(daemon.as_ref(), &stream, desc);
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
