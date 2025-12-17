pub mod text_document;

use crate::{Server, message::Request};
use lsp_types::{
    CompletionOptions, DiagnosticOptions, DiagnosticServerCapabilities, InitializeResult,
    ServerCapabilities, ServerInfo, TextDocumentSyncCapability, TextDocumentSyncKind,
    WorkDoneProgressOptions,
    request::{Initialize, Request as LspRequest, Shutdown},
};

pub fn initialize() -> <Initialize as LspRequest>::Result {
    return InitializeResult {
        capabilities: ServerCapabilities {
            completion_provider: Some(CompletionOptions {
                resolve_provider: None,
                trigger_characters: None,
                all_commit_characters: None,
                work_done_progress_options: WorkDoneProgressOptions::default(),
                completion_item: None,
            }),
            text_document_sync: Some(TextDocumentSyncCapability::Kind(
                TextDocumentSyncKind::INCREMENTAL,
            )),
            diagnostic_provider: Some(DiagnosticServerCapabilities::Options(DiagnosticOptions {
                identifier: None,
                inter_file_dependencies: true,
                workspace_diagnostics: false,
                work_done_progress_options: WorkDoneProgressOptions::default(),
            })),
            ..Default::default()
        },
        server_info: Some(ServerInfo {
            name: "adept".into(),
            version: Some("3.0.0".into()),
        }),
    };
}

pub fn shutdown(server: &mut Server, _request: Request) -> <Shutdown as LspRequest>::Result {
    server.did_shutdown = true;
}
