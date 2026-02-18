use crate::{Static, methods::Forward};

impl Forward for Static<lsp_types::request::ExecuteCommand> {
    const IS_REQUEST: bool = true;
}
