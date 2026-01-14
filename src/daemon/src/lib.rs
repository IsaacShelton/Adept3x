mod connection;
mod daemon;
mod logger;
mod queue;

pub use crate::{connection::Connection, daemon::Daemon};
use file_cache::{FileCache, FileContent};
#[cfg(target_family = "unix")]
use lsp_message::LspMessage;
pub use queue::*;
#[cfg(target_family = "unix")]
use std::os::unix::net::{SocketAddr, UnixStream};
use std::{io, sync::Arc};
#[cfg(target_family = "unix")]
use std::{io::BufReader, io::ErrorKind, time::Duration};

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
    }

    #[cfg(target_family = "windows")]
    {
        let _ = daemon;
        panic!("Windows is not supported")
    }
}

pub struct Client {
    file_cache: FileCache,
}

impl Client {
    pub fn new() -> Self {
        Self {
            file_cache: FileCache::default(),
        }
    }
}

#[cfg(target_family = "unix")]
fn handle_client(_daemon: Arc<Daemon>, stream: UnixStream, address: SocketAddr) {
    use file_cache::Canonical;

    log::info!("Accepted client {:?} {:?}", stream, address);
    std::thread::sleep(Duration::from_millis(50));

    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    let reader = &mut BufReader::new(&stream);

    let mut client = Client::new();

    let config_filepath = match std::env::current_dir()
        .map_err(|_| ())
        .and_then(|path| Canonical::new(path.join("adept.build")))
    {
        Ok(config_filepath) => {
            log::info!("Found config file {:?}", config_filepath);
            config_filepath
        }
        Err(error) => {
            log::error!("Failed to find config file - {:?}", error);
            return;
        }
    };

    let config_text =
        std::fs::read_to_string(config_filepath.as_path()).expect("Failed to read config file");

    let config_file_id = client.file_cache.preregister_file(config_filepath);
    log::info!("Config file id is {:?}", config_file_id);
    log::info!("Config text is {}", config_text);
    client
        .file_cache
        .set_content(config_file_id, FileContent::Text(config_text));

    loop {
        match LspMessage::read(reader) {
            Ok(None) => {
                log::info!("Shutting down connection to client");
                break;
            }
            Ok(Some(LspMessage::Notification(notification))) => {
                log::info!("Got notification {:?}", notification);
            }
            Ok(Some(LspMessage::Request(request))) => {
                log::info!("Got request {:?}", request);
            }
            Ok(Some(LspMessage::Response(response))) => {
                log::info!("Got response {:?}", response);
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
