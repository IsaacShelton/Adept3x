use crate::{LspEndpoint, MaybeReady, Static};
use lsp_connection::{LspConnection, LspConnectionState};

impl LspEndpoint for Static<lsp_types::request::Shutdown> {
    fn run(client: &mut LspConnection, _params: Self::Params) -> MaybeReady<Self::Result> {
        client.connection_state = LspConnectionState::Shutdown;
        MaybeReady::Ready(())
    }
}
