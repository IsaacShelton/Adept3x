use crate::{LspMethod, MaybeReady};
use lsp_connection::LspConnection;

pub trait LspEndpoint: LspMethod {
    fn run(client: &mut LspConnection, params: Self::Params) -> MaybeReady<Self::Result>;
}
