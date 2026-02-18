use crate::{Static, methods::Forward};

impl Forward for Static<lsp_types::notification::DidOpenTextDocument> {
    const IS_REQUEST: bool = false;
}
