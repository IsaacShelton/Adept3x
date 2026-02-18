use crate::{Static, methods::Forward};

impl Forward for Static<lsp_types::request::DocumentDiagnosticRequest> {
    const IS_REQUEST: bool = true;
}
