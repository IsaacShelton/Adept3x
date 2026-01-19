use crate::{LspEndpoint, MaybeReady, NeverRespond, Static};
use lsp_connection::{LspConnection, LspConnectionState};
use lsp_message::LspRequestId;

impl LspEndpoint for Static<lsp_types::notification::Initialized> {
    fn run(
        client: &mut LspConnection,
        _id: Option<&LspRequestId>,
        _params: Self::Params,
    ) -> MaybeReady<Self::Result> {
        log::info!("Server and client are now initialized");
        client.state = LspConnectionState::Initialized;
        MaybeReady::Ready(NeverRespond)
    }
}
