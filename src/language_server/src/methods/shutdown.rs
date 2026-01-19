use crate::{LspEndpoint, MaybeReady, Static};
use lsp_connection::{LspConnection, LspConnectionState};
use lsp_message::LspRequestId;

impl LspEndpoint for Static<lsp_types::request::Shutdown> {
    fn run(
        client: &mut LspConnection,
        _id: Option<&LspRequestId>,
        _params: Self::Params,
    ) -> MaybeReady<Self::Result> {
        client.state = LspConnectionState::Shutdown;
        MaybeReady::Ready(())
    }
}
