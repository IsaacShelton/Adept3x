use crate::{LspRequestEndpoint, MaybeReady, Static};
use lsp_connection::{LspConnection, LspConnectionState};

impl LspRequestEndpoint for Static<lsp_types::request::Shutdown> {
    fn run(client: &mut LspConnection, _params: Self::Params) -> MaybeReady<Self::Result> {
        client.connection_state = LspConnectionState::Shutdown;
        MaybeReady::Ready(())
    }
}
