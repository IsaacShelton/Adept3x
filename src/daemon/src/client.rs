#[cfg(target_family = "unix")]
use crate::Daemon;
use document::Document;
use file_cache::{Canonical, FileBytes, FileCache, FileContent, FileId, FileKind};
use file_uri::DecodeFileUri;
#[cfg(target_family = "unix")]
use lsp_message::LspMessage;
use lsp_message::{LspNotification, LspRequest, LspRequestId, LspResponse};
use lsp_types::{
    CompletionItem, CompletionList, CompletionParams, CompletionResponse, Diagnostic,
    DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
    ExecuteCommandParams, FullDocumentDiagnosticReport, Position, Range,
    RelatedFullDocumentDiagnosticReport, Uri,
};
#[cfg(target_family = "unix")]
use std::os::unix::net::{SocketAddr, UnixStream};
use std::{borrow::Cow, ffi::OsStr, path::PathBuf, str::FromStr, sync::Arc};
#[cfg(target_family = "unix")]
use std::{io::BufReader, io::ErrorKind, time::Duration};
use syntax_tree::BareSyntaxKind;
use text_edit::TextEditOrFullUtf16;

pub struct Client {
    file_cache: FileCache,
    next_request_id: LspRequestId,
    config_file: ConfigFile,
}

pub enum ConfigFile {
    Missing,
    Prompted(LspRequestId),
    Present(FileId),
}

impl Client {
    pub fn new() -> Self {
        Self {
            file_cache: FileCache::default(),
            next_request_id: LspRequestId::Int(0),
            config_file: ConfigFile::Missing,
        }
    }

    pub fn next_request_id(&mut self) -> LspRequestId {
        let id = self.next_request_id.clone();
        self.next_request_id = self.next_request_id.succ();
        id
    }
}

#[cfg(target_family = "unix")]
pub fn handle_client(daemon: &Daemon, stream: UnixStream, address: SocketAddr) {
    log::info!("Accepted client {:?} {:?}", stream, address);

    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();

    let reader = &mut BufReader::new(&stream);
    let mut client = Client::new();
    client.config_file = get_config_file_id(&mut client, &stream);

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
                if let ConfigFile::Present(_) = &client.config_file {
                    daemon.idle_tracker.still_active();
                    log::info!("Got request {:?}", request);
                }

                let response_or_original_request = on_request::<
                    lsp_types::request::DocumentDiagnosticRequest,
                >(request, |id, params| {
                    document_diagnostics_request(&mut client, id, params)
                })
                .or_else(|request| {
                    on_request::<lsp_types::request::Completion>(request, |id, params| {
                        completion(&mut client, id, params)
                    })
                })
                .or_else(|request| {
                    on_request::<lsp_types::request::ExecuteCommand>(request, |id, params| {
                        execute_command(&mut client, id, params)
                    })
                })
                .or_else(|request| {
                    log::warn!("Unhandled request {:?}", request);
                    Err(request)
                });

                if let Ok(response) = response_or_original_request {
                    response
                        .write(&mut &stream)
                        .expect("Failed to send message to client");
                }
            }
            Ok(Some(LspMessage::Response(response))) => {
                if let ConfigFile::Present(_) = &client.config_file {
                    daemon.idle_tracker.still_active();
                    log::info!("Got response {:?}", response);
                }

                if let ConfigFile::Prompted(config_file_lsp_request_id) = &client.config_file {
                    if response.id == *config_file_lsp_request_id {
                        if let Some(choice) = response.result.and_then(|value| {
                            serde_json::from_value::<lsp_types::MessageActionItem>(value).ok()
                        }) {
                            log::error!("They chose {:?}", choice.title);
                            client.config_file = ConfigFile::Missing;
                        } else {
                            client.config_file = ConfigFile::Missing;
                        }
                    }
                }
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

#[cfg(target_family = "unix")]
fn get_config_file_id(client: &mut Client, stream: &UnixStream) -> ConfigFile {
    match std::env::current_dir()
        .map_err(|_| ())
        .and_then(|path| Canonical::new(path.join("adept.build")))
    {
        Ok(config_filepath) => {
            use document::Document;
            use std::borrow::Cow;

            log::info!("Found config file {:?}", config_filepath);
            let config_text = std::fs::read_to_string(config_filepath.as_path())
                .expect("Failed to read config file");

            let config_file_id = client
                .file_cache
                .preregister_file(Cow::Owned(config_filepath));

            log::info!("Config file id is {:?}", config_file_id);
            log::info!("Config text is {}", config_text);

            let document = Document::new(config_text.into());

            let syntax_tree = parser_adept::reparse(&document, None, document.full_range());

            log::error!("Got syntax tree {:?}", syntax_tree);

            let file_bytes = FileBytes::Document(document);

            client.file_cache.set_content(
                config_file_id,
                FileContent {
                    kind: FileKind::ProjectConfig,
                    file_bytes,
                    syntax_tree: Some(syntax_tree),
                },
            );

            ConfigFile::Present(config_file_id)
        }
        Err(error) => {
            log::error!("Failed to find config file - {:?}", error);

            let create_project_file_prompt_request_id = client.next_request_id();

            crate::show::show_message_request(
                &stream,
                create_project_file_prompt_request_id.clone(),
                lsp_types::MessageType::INFO,
                "Missing `adept.build` project config file!".into(),
                ["Create".into(), "Ignore".into(), "Another".into()].into_iter(),
            );

            ConfigFile::Prompted(create_project_file_prompt_request_id)
        }
    }
}

fn on_request<T: lsp_types::request::Request>(
    request: LspRequest,
    then: impl FnOnce(&LspRequestId, T::Params) -> Result<T::Result, LspResponse>,
) -> Result<LspMessage, LspRequest> {
    if request.method.as_str() != T::METHOD {
        return Err(request);
    }

    let response = then(
        &request.id,
        serde_json::from_value(request.params).expect("invalid request"),
    );

    let response = match response {
        Ok(result) => LspResponse {
            id: request.id,
            result: Some(serde_json::to_value(result).expect("response is serializable")),
            error: None,
        },
        Err(response) => response,
    };

    Ok(LspMessage::Response(response))
}

fn document_diagnostics_request(
    client: &mut Client,
    _id: &LspRequestId,
    params: DocumentDiagnosticParams,
) -> Result<DocumentDiagnosticReportResult, LspResponse> {
    let mut diagnostics = vec![];

    if let Some(filepath) = params.text_document.uri.decode_file_uri() {
        if let Ok(filepath) = Canonical::new(filepath) {
            let file_id = client.file_cache.preregister_file(Cow::Owned(filepath));

            if let Some(file_content) = client.file_cache.get_content(file_id) {
                if let Some(syntax_tree) = &file_content.syntax_tree {
                    let bindings = syntax_tree
                        .bare()
                        .children()
                        .filter(|x| matches!(x.kind(), BareSyntaxKind::Binding));

                    let mut binding_names = bindings
                        .flat_map(|binding| {
                            binding
                                .children()
                                .find(|child| matches!(child.kind(), BareSyntaxKind::Name))
                                .map(|name| {
                                    name.children().find_map(|id| match id.kind() {
                                        BareSyntaxKind::Identifier(name) => Some(name),
                                        _ => None,
                                    })
                                })
                        })
                        .flatten();

                    let names = itertools::Itertools::join(&mut binding_names, ", ");

                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: 1,
                                character: 0,
                            },
                            end: Position {
                                line: 1,
                                character: 4,
                            },
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        source: Some("Adept".into()),
                        message: format!("Defined bindings are: {}", names),
                        related_information: None,
                        tags: None,
                        data: None,
                        ..Default::default()
                    });
                }
            }
        }
    }

    Ok(DocumentDiagnosticReportResult::Report(
        DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
            related_documents: None,
            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                result_id: None,
                items: diagnostics,
            },
        }),
    ))
}

fn completion(
    _client: &mut Client,
    _id: &LspRequestId,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>, LspResponse> {
    Ok(Some(CompletionResponse::List(CompletionList {
        is_incomplete: true,
        items: vec![CompletionItem {
            label: "testing_word".into(),
            ..Default::default()
        }],
    })))
}

fn get_file_content<'c, 'u>(
    client: &'c mut Client,
    uri: &'c Uri,
) -> Option<(Arc<FileContent>, FileId, Canonical<PathBuf>)> {
    uri.decode_file_uri()
        .and_then(|filepath| Canonical::new(filepath).ok())
        .and_then(|filepath| {
            let file_id = client.file_cache.preregister_file(Cow::Borrowed(&filepath));
            client
                .file_cache
                .get_content(file_id)
                .map(|file_content| (file_content, file_id, filepath))
        })
}

fn execute_command(
    client: &mut Client,
    _id: &LspRequestId,
    params: ExecuteCommandParams,
) -> Result<Option<serde_json::Value>, LspResponse> {
    match params.command.as_str() {
        "adept.showSyntaxTree" => {
            let result = params
                .arguments
                .first()
                .and_then(|arg| arg.as_str())
                .and_then(|arg| Some(lsp_types::Uri::from_str(arg)))
                .into_iter()
                .flatten()
                .next()
                .as_ref()
                .and_then(|uri| get_file_content(client, uri))
                .and_then(|file_content| {
                    file_content
                        .0
                        .as_ref()
                        .syntax_tree
                        .as_ref()
                        .map(|syntax_tree| {
                            let mut value = Vec::new();
                            let _ = syntax_tree.dump(&mut value, 0);
                            String::from_utf8(value).ok()
                        })
                })
                .map(|string| serde_json::Value::from(string))
                .unwrap_or_else(|| serde_json::Value::from(""));

            Ok(Some(result))
        }
        _ => Ok(None),
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

            let file_bytes = FileBytes::Document(Document::new(params.text_document.text.into()));
            let file_id = client.file_cache.preregister_file(Cow::Owned(filepath));
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
    let Some((file_content, file_id, filepath)) =
        get_file_content(client, &params.text_document.uri)
    else {
        return;
    };

    log::info!("Change for {:?}", filepath);

    if let Some(_) = file_content.file_bytes.as_document() {
        let edits = params
            .content_changes
            .into_iter()
            .map(TextEditOrFullUtf16::from);

        let mut file_content = file_content.after_edits(std::iter::empty());

        for edit in edits {
            file_content = file_content.after_edits(std::iter::once(edit));

            if let Some(document) = file_content.file_bytes.as_document() {
                let old_syntax_tree = file_content.syntax_tree.clone();
                let new_syntax_tree =
                    parser_adept::reparse(document, old_syntax_tree, document.full_range());
                file_content.syntax_tree = Some(new_syntax_tree);
            }
        }

        client.file_cache.set_content(file_id, file_content);
    }
}
