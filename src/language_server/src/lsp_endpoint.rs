use crate::{LspMethod, MaybeReady};
use lsp_connection::LspConnection;
use lsp_message::LspRequestId;

pub trait LspEndpoint: LspMethod {
    fn run(
        client: &mut LspConnection,
        id: Option<&LspRequestId>,
        params: Self::Params,
    ) -> MaybeReady<Self::Result>;
}
