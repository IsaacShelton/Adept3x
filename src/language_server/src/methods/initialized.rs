use crate::{LspEndpoint, MaybeReady, NeverRespond, Static};
use lsp_connection::{LspConnection, LspConnectionState};

impl LspEndpoint for Static<lsp_types::notification::Initialized> {
    fn run(client: &mut LspConnection, _params: Self::Params) -> MaybeReady<Self::Result> {
        log::info!("Server and client are now initialized");
        client.connection_state = LspConnectionState::Initialized;
        MaybeReady::Ready(NeverRespond)
    }
}
