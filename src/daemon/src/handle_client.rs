use crate::Daemon;
use document::Document;
use file_cache::{Canonical, FileBytes, FileCache, FileContent, FileId, FileKind};
use file_uri::DecodeFileUri;
use lsp_message::{LspMessage, LspNotification, LspRequest, LspRequestId, LspResponse};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, CompletionParams, CompletionResponse,
    Diagnostic, DiagnosticSeverity, DidChangeTextDocumentParams, DidOpenTextDocumentParams,
    DocumentDiagnosticParams, DocumentDiagnosticReport, DocumentDiagnosticReportResult,
    ExecuteCommandParams, FullDocumentDiagnosticReport, RelatedFullDocumentDiagnosticReport, Uri,
};
use std::{
    borrow::Cow,
    ffi::OsStr,
    io::{BufReader, ErrorKind, Read, Write},
    panic::catch_unwind,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};
use syntax_tree::{BareSyntaxKind, BuiltinType};
use text_edit::TextEditOrFullUtf16;

pub struct Client {
    file_cache: FileCache,
    #[allow(unused)]
    next_request_id: LspRequestId,
    config_file: ConfigFile,
}

impl Client {
    pub fn get_file_content(
        &mut self,
        uri: &Uri,
    ) -> Option<(Arc<FileContent>, FileId, Canonical<PathBuf>)> {
        uri.decode_file_uri()
            .and_then(|filepath| Canonical::new(filepath).ok())
            .and_then(|filepath| {
                let file_id = self.file_cache.preregister_file(Cow::Borrowed(&filepath));
                self.file_cache
                    .get_content(file_id)
                    .map(|file_content| (file_content, file_id, filepath))
            })
    }
}

pub enum ConfigFile {
    Missing,
    #[allow(unused)]
    Prompted(LspRequestId),
    #[allow(unused)]
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

    #[allow(unused)]
    pub fn next_request_id(&mut self) -> LspRequestId {
        let id = self.next_request_id.clone();
        self.next_request_id = self.next_request_id.succ();
        id
    }
}

pub fn handle_client(daemon: &Daemon, mut stream: impl Read + Write + Copy, desc: String) {
    log::info!("Accepted client {:?}", desc);

    let reader = &mut BufReader::new(stream);
    let mut client = Client::new();
    client.config_file = ConfigFile::Missing; // get_config_file_id(&mut client, &stream);

    loop {
        match LspMessage::read(reader) {
            Ok(None) => {
                log::info!("Done handling client");
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
                        .write(&mut stream)
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
            Ok(Some(LspMessage::Compile(compile))) => {
                log::info!("Compiling {}", compile.filename);
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

    let Some((file_content, _, _)) = client.get_file_content(&params.text_document.uri) else {
        return Ok(DocumentDiagnosticReportResult::Report(
            DocumentDiagnosticReport::Full(RelatedFullDocumentDiagnosticReport {
                related_documents: None,
                full_document_diagnostic_report: FullDocumentDiagnosticReport {
                    result_id: None,
                    items: diagnostics,
                },
            }),
        ));
    };

    if let Some(syntax_tree) = &file_content.syntax_tree {
        let mut stack = Vec::from_iter(syntax_tree.children());

        while let Some(node) = stack.pop() {
            stack.extend(node.children());

            if let BareSyntaxKind::Error { description } = node.bare().kind() {
                let range = node.text_range();

                diagnostics.push(Diagnostic {
                    range: range.into(),
                    severity: Some(DiagnosticSeverity::ERROR),
                    source: Some("Adept".into()),
                    message: description.into(),
                    related_information: None,
                    tags: None,
                    data: None,
                    ..Default::default()
                });
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
    client: &mut Client,
    _id: &LspRequestId,
    params: CompletionParams,
) -> Result<Option<CompletionResponse>, LspResponse> {
    let text_document = &params.text_document_position.text_document;
    let mut items = vec![];

    let Some((file_content, _, _)) = client.get_file_content(&text_document.uri) else {
        return Ok(Some(CompletionResponse::List(CompletionList {
            is_incomplete: true,
            items,
        })));
    };

    struct BindingInfo<'a> {
        name: &'a str,
        kind: Option<CompletionItemKind>,
    }

    if let Some(syntax_tree) = &file_content.syntax_tree {
        let binding_names = syntax_tree
            .bare()
            .children()
            .filter(|x| matches!(x.kind(), BareSyntaxKind::Binding))
            .flat_map(|binding| {
                let name = binding
                    .children()
                    .find(|child| matches!(child.kind(), BareSyntaxKind::Name))
                    .and_then(|name| {
                        name.children().find_map(|id| match id.kind() {
                            BareSyntaxKind::Identifier(name) => Some(name),
                            _ => None,
                        })
                    });

                let kind = binding
                    .children()
                    .find_map(|child| match child.kind() {
                        BareSyntaxKind::Term => Some(child),
                        _ => None,
                    })
                    .and_then(|term| {
                        term.children().find_map(|child| match child.kind() {
                            BareSyntaxKind::BuiltinType(BuiltinType::Fn) => {
                                Some(CompletionItemKind::INTERFACE)
                            }
                            BareSyntaxKind::BuiltinType(BuiltinType::Record) => {
                                Some(CompletionItemKind::STRUCT)
                            }
                            BareSyntaxKind::BuiltinType(
                                BuiltinType::Bool
                                | BuiltinType::Void
                                | BuiltinType::Nat
                                | BuiltinType::Type,
                            ) => Some(CompletionItemKind::ENUM),
                            BareSyntaxKind::FnValue => Some(CompletionItemKind::FUNCTION),
                            BareSyntaxKind::TrueValue
                            | BareSyntaxKind::FalseValue
                            | BareSyntaxKind::VoidValue
                            | BareSyntaxKind::IfValue
                            | BareSyntaxKind::Block
                            | BareSyntaxKind::Variable(_) => Some(CompletionItemKind::VALUE),
                            _ => None,
                        })
                    });

                name.map(|name| BindingInfo { name, kind })
            });

        items.extend(binding_names.map(|info| CompletionItem {
            label: info.name.to_string(),
            kind: info.kind,
            ..Default::default()
        }));
    }

    items.extend([
        CompletionItem {
            label: "Void".into(),
            kind: Some(CompletionItemKind::ENUM),
            ..Default::default()
        },
        CompletionItem {
            label: "Bool".into(),
            kind: Some(CompletionItemKind::ENUM),
            ..Default::default()
        },
        CompletionItem {
            label: "Type".into(),
            kind: Some(CompletionItemKind::ENUM),
            ..Default::default()
        },
        CompletionItem {
            label: "void".into(),
            kind: Some(CompletionItemKind::VALUE),
            ..Default::default()
        },
        CompletionItem {
            label: "true".into(),
            kind: Some(CompletionItemKind::VALUE),
            ..Default::default()
        },
        CompletionItem {
            label: "false".into(),
            kind: Some(CompletionItemKind::VALUE),
            ..Default::default()
        },
    ]);

    Ok(Some(CompletionResponse::List(CompletionList {
        is_incomplete: true,
        items,
    })))
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
                .map(|arg| lsp_types::Uri::from_str(arg))
                .into_iter()
                .flatten()
                .next()
                .as_ref()
                .and_then(|uri| client.get_file_content(uri))
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
        "adept.debugEval" => {
            let result_string = params
                .arguments
                .first()
                .and_then(|arg| arg.as_str())
                .map(|arg| lsp_types::Uri::from_str(arg))
                .into_iter()
                .flatten()
                .next()
                .as_ref()
                .and_then(|uri| client.get_file_content(uri))
                .and_then(|file_content| {
                    file_content
                        .0
                        .as_ref()
                        .syntax_tree
                        .as_ref()
                        .map(|syntax_tree| {
                            catch_unwind(|| kernel::debug_eval(syntax_tree))
                                .unwrap_or_else(|e| format!("<paniced>: {e:?}"))
                        })
                })
                .unwrap_or_else(|| "".into());

            Ok(Some(serde_json::Value::from(result_string)))
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

            let document = Document::new(&params.text_document.text);
            let syntax_tree = parser_adept::reparse(&document, None, document.full_range());

            let file_bytes = FileBytes::Document(document);
            let file_id = client.file_cache.preregister_file(Cow::Owned(filepath));
            log::info!("on_notif did open {:?} {:?}", file_id, &file_bytes);

            client.file_cache.set_content(
                file_id,
                file_cache::FileContent {
                    kind,
                    file_bytes,
                    syntax_tree: Some(syntax_tree),
                },
            );
        }
    }
}

fn did_change(client: &mut Client, params: DidChangeTextDocumentParams) {
    let Some((file_content, file_id, _filepath)) =
        client.get_file_content(&params.text_document.uri)
    else {
        return;
    };

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
