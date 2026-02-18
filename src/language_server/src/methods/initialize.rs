use crate::{LspEndpoint, MaybeReady, Static};
use lsp_connection::LspConnection;
use lsp_message::LspRequestId;
use lsp_types::{
    CompletionOptions, DiagnosticOptions, DiagnosticServerCapabilities, ExecuteCommandOptions,
    InitializeResult, ServerCapabilities, ServerInfo, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextDocumentSyncOptions, TextDocumentSyncSaveOptions,
    WorkDoneProgressOptions,
};

impl LspEndpoint for Static<lsp_types::request::Initialize> {
    fn run(
        _client: &mut LspConnection,
        _id: Option<&LspRequestId>,
        params: Self::Params,
    ) -> MaybeReady<Self::Result> {
        let save = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|text_document| text_document.synchronization.as_ref())
            .and_then(|synchronization| {
                synchronization
                    .did_save
                    .map(TextDocumentSyncSaveOptions::Supported)
            });

        let completion_provider = params
            .capabilities
            .text_document
            .as_ref()
            .and_then(|text_document| text_document.completion.as_ref())
            .map(|_| CompletionOptions {
                ..Default::default()
            });

        MaybeReady::Ready(InitializeResult {
            capabilities: ServerCapabilities {
                position_encoding: None,
                completion_provider,
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        // We must support none or all three of didOpen, didClose, and didChange.
                        // We don't have to check for compatability for these, since
                        // the language client is required to implement them.
                        // https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_synchronization
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::INCREMENTAL),
                        will_save: None,
                        will_save_wait_until: None,
                        save,
                    },
                )),
                diagnostic_provider: Some(DiagnosticServerCapabilities::Options(
                    DiagnosticOptions {
                        identifier: None,
                        inter_file_dependencies: true,
                        workspace_diagnostics: false,
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                    },
                )),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["Show Syntax Tree".into()],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "adept".into(),
                version: Some("3.0.0".into()),
            }),
        })
    }
}
