#[cfg(target_family = "unix")]
use crate::Daemon;
use file_cache::{Canonical, FileBytes, FileCache, FileKind};
use file_uri::DecodeFileUri;
#[cfg(target_family = "unix")]
use lsp_message::LspMessage;
use lsp_message::LspNotification;
use lsp_types::{DidChangeTextDocumentParams, DidOpenTextDocumentParams};
use std::ffi::OsStr;
#[cfg(target_family = "unix")]
use std::os::unix::net::{SocketAddr, UnixStream};
#[cfg(target_family = "unix")]
use std::{io::BufReader, io::ErrorKind, time::Duration};
use text_edit::TextEditOrFullUtf16;

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
pub fn handle_client(daemon: &Daemon, stream: UnixStream, address: SocketAddr) {
    use file_cache::{FileContent, FileKind};

    log::info!("Accepted client {:?} {:?}", stream, address);

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

    {
        let config_text =
            std::fs::read_to_string(config_filepath.as_path()).expect("Failed to read config file");

        let config_file_id = client.file_cache.preregister_file(config_filepath);
        log::info!("Config file id is {:?}", config_file_id);
        log::info!("Config text is {}", config_text);

        let file_bytes = FileBytes::Text(config_text);
        client.file_cache.set_content(
            config_file_id,
            FileContent {
                kind: FileKind::ProjectConfig,
                file_bytes,
                syntax_tree: None,
            },
        );
    }

    loop {
        match LspMessage::read(reader) {
            Ok(None) => {
                log::info!("Shutting down connection to client");
                break;
            }
            Ok(Some(LspMessage::Notification(notification))) => {
                daemon.idle_tracker.still_active();

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
                daemon.idle_tracker.still_active();
                log::info!("Got request {:?}", request);
            }
            Ok(Some(LspMessage::Response(response))) => {
                daemon.idle_tracker.still_active();
                log::info!("Got response {:?}", response);
            }
            Err(error) => {
                if let ErrorKind::WouldBlock = error.kind() {
                    // No message is ready to receive from the client yet
                } else {
                    daemon.idle_tracker.still_active();
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
            let is_adept = filepath.extension() == Some(OsStr::new("adept"));
            let kind = if is_adept {
                FileKind::Adept
            } else {
                FileKind::Unknown
            };
            let file_bytes = FileBytes::Text(params.text_document.text.into());
            let file_id = client.file_cache.preregister_file(filepath);
            log::info!("on_notif did open {:?} {:?}", file_id, &file_bytes);

            client.file_cache.set_content(
                file_id,
                file_cache::FileContent {
                    kind,
                    file_bytes,
                    syntax_tree: None,
                },
            );
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
