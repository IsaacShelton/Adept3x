use crate::{
    DocumentBody, Server,
    message::{Notification, Request},
};
use lsp_types::{
    CompletionItem, CompletionList, CompletionParams, CompletionResponse, Diagnostic,
    DiagnosticSeverity, DidChangeTextDocumentParams, DocumentDiagnosticParams,
    DocumentDiagnosticReport, DocumentDiagnosticReportResult, FullDocumentDiagnosticReport,
    Position, Range, RelatedFullDocumentDiagnosticReport,
    request::{Completion, DocumentDiagnosticRequest, Request as LspRequest},
};

pub fn did_change(server: &mut Server, request: Notification) {
    let mut params = serde_json::from_value::<DidChangeTextDocumentParams>(request.params).unwrap();

    if let Some(change) = params.content_changes.pop() {
        server.documents.insert(
            params.text_document.uri,
            DocumentBody {
                content: change.text,
            },
        );
    }
}

pub fn completion(server: &Server, request: Request) -> <Completion as LspRequest>::Result {
    let params = serde_json::from_value::<CompletionParams>(request.params).unwrap();

    let body = server
        .documents
        .get(&params.text_document_position.text_document.uri)?;

    let mut word = body.get_word_at(params.text_document_position.position)?;

    if word == "" {
        word = "what";
    }

    return Some(CompletionResponse::List(CompletionList {
        is_incomplete: true,
        items: vec![CompletionItem {
            label: format!("{}!!!", word),
            ..Default::default()
        }],
    }));
}

pub fn diagnostic(
    _server: &Server,
    request: Request,
) -> <DocumentDiagnosticRequest as LspRequest>::Result {
    let _params = serde_json::from_value::<DocumentDiagnosticParams>(request.params).unwrap();

    let diagnostics = vec![
        Diagnostic {
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
            message: "This is an error, you should fix it.".into(),
            related_information: None,
            tags: None,
            data: None,
            ..Default::default()
        },
        Diagnostic {
            range: Range {
                start: Position {
                    line: 2,
                    character: 0,
                },
                end: Position {
                    line: 2,
                    character: 4,
                },
            },
            severity: Some(DiagnosticSeverity::WARNING),
            source: Some("Adept".into()),
            message: "This is warning, you might want to fix it.".into(),
            related_information: None,
            tags: None,
            data: None,
            ..Default::default()
        },
    ];

    DocumentDiagnosticReportResult::Report(DocumentDiagnosticReport::Full(
        RelatedFullDocumentDiagnosticReport {
            related_documents: None,
            full_document_diagnostic_report: FullDocumentDiagnosticReport {
                result_id: None,
                items: diagnostics,
            },
        },
    ))
}
