mod connection;
mod daemon;
mod logger;
mod queue;

pub use crate::{connection::Connection, daemon::Daemon};
use file_cache::{Canonical, FileCache, FileContent};
use file_uri::DecodeFileUri;
#[cfg(target_family = "unix")]
use lsp_message::LspMessage;
use lsp_message::LspNotification;
use lsp_types::{DidChangeTextDocumentParams, DidOpenTextDocumentParams};
pub use queue::*;
#[cfg(target_family = "unix")]
use std::os::unix::net::{SocketAddr, UnixStream};
use std::{ffi::OsStr, io, sync::Arc};
#[cfg(target_family = "unix")]
use std::{io::BufReader, io::ErrorKind, time::Duration};
use text_edit::TextEditOrFullUtf16;

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
                let _ = on_notif::<lsp_types::notification::DidOpenTextDocument>(
                    notification,
                    |params| did_open(&mut client, params),
                )
                .or_else(|notification| {
                    on_notif::<lsp_types::notification::DidChangeTextDocument>(
                        notification,
                        |params| did_change(&mut client, params),
                    )
                })
                .or_else(|notification| {
                    log::warn!("Unhandled notification {:?}", notification);
                    Result::<(), ()>::Ok(())
                });
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

fn on_notif<T: lsp_types::notification::Notification>(
    notification: LspNotification,
    then: impl FnOnce(T::Params),
) -> Result<(), LspNotification> {
    if notification.method.as_str() != T::METHOD {
        return Err(notification);
    }

    then(serde_json::from_value(notification.params).expect("invalid notification"));
    Ok(())
}

fn did_open(client: &mut Client, params: DidOpenTextDocumentParams) {
    if let Some(filepath) = params.text_document.uri.decode_file_uri() {
        if let Ok(filepath) = Canonical::new(filepath) {
            let _is_adept = filepath.extension() == Some(OsStr::new("adept"));
            let file_content = FileContent::Text(params.text_document.text.into());
            let file_id = client.file_cache.preregister_file(filepath);
            log::info!("on_notif did open {:?} {:?}", file_id, &file_content);
            client.file_cache.set_content(file_id, file_content);
        }
    }
}

fn did_change(client: &mut Client, params: DidChangeTextDocumentParams) {
    let Some(filepath) = params.text_document.uri.decode_file_uri() else {
        return;
    };

    let Ok(filepath) = Canonical::new(filepath) else {
        return;
    };

    log::info!("Change for {:?}", filepath);
    let file_id = client.file_cache.preregister_file(filepath);
    let Some(file_content) = client.file_cache.get_content(file_id) else {
        return;
    };

    let edits = params
        .content_changes
        .into_iter()
        .map(TextEditOrFullUtf16::from);

    client
        .file_cache
        .set_content(file_id, file_content.after_edits(edits));
    log::info!("Existing is {:?} {:?}", file_id, file_content);
    log::info!(
        "New content is {:?} {:?}",
        file_id,
        client.file_cache.get_content(file_id)
    );
}
