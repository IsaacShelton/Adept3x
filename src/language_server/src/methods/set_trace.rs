use crate::{LspEndpoint, MaybeReady, NeverRespond, Static};
use lsp_connection::LspConnection;
use lsp_message::LspRequestId;

impl LspEndpoint for Static<lsp_types::notification::SetTrace> {
    fn run(
        _client: &mut LspConnection,
        _id: Option<&LspRequestId>,
        _params: Self::Params,
    ) -> MaybeReady<Self::Result> {
        MaybeReady::Ready(NeverRespond)
    }
}
