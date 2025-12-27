mod connection;
mod daemon;
mod logger;
mod queue;

pub use crate::{connection::Connection, daemon::Daemon};
use lsp_message::LspMessage;
pub use queue::*;
#[cfg(target_family = "unix")]
use std::os::unix::net::{SocketAddr, UnixStream};
use std::{
    io::{self, BufReader, ErrorKind},
    sync::Arc,
    time::Duration,
};

pub fn main_loop(daemon: Daemon) -> io::Result<()> {
    let daemon = Arc::new(daemon);

    if logger::setup().is_err() {
        eprintln!("Failed to create daemon log file");
    }

    daemon
        .listener
        .set_nonblocking(true)
        .expect("Failed to set to non-blocking");

    // Executor thread
    std::thread::spawn(|| {});

    // Accept clients
    #[cfg(target_family = "unix")]
    loop {
        match daemon.listener.accept() {
            Ok((stream, address)) => {
                let daemon = Arc::clone(&daemon);
                std::thread::spawn(move || handle_client(daemon, stream, address));
            }
            Err(error) => {
                if let io::ErrorKind::WouldBlock = error.kind() {
                    // No clients ready to connect to us yet
                } else {
                    log::error!("Failed to accept client: {:?}", error);
                }
            }
        }

        if daemon.should_exit() {
            return Ok(());
        }

        std::thread::sleep(Duration::from_millis(50));
    }

    #[cfg(target_family = "windows")]
    panic!("Windows is not supported")
}

#[cfg(target_family = "unix")]
fn handle_client(_daemon: Arc<Daemon>, stream: UnixStream, address: SocketAddr) {
    log::info!("Accepted client {:?} {:?}", stream, address);
    std::thread::sleep(Duration::from_millis(50));

    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    let reader = &mut BufReader::new(&stream);

    loop {
        match LspMessage::read(reader) {
            Ok(None) => {
                log::info!("Shutting down connection to client");
                break;
            }
            Ok(message) => {
                log::info!("Got message {:?}", message);
            }
            Err(error) => {
                if let ErrorKind::WouldBlock = error.kind() {
                    // Nothing to do
                } else {
                    log::error!("Error receiving message from client - {:?}", error);
                }
            }
        }
    }
}
