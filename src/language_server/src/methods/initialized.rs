use crate::{LspRequestEndpoint, MaybeReady, NeverRespond, Static};
use lsp_connection::LspConnection;

impl LspRequestEndpoint for Static<lsp_types::notification::Initialized> {
    fn run(_client: &mut LspConnection, _params: Self::Params) -> MaybeReady<Self::Result> {
        MaybeReady::Ready(NeverRespond)
    }
}
