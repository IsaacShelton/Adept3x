use crate::{LspEndpoint, LspMethod, MaybeReady, NeverRespond, Static};
use lsp_connection::LspConnection;
use lsp_message::{LspNotification, LspRequestId};

impl LspEndpoint for Static<lsp_types::notification::DidOpenTextDocument> {
    fn run(
        client: &mut LspConnection,
        _id: Option<&LspRequestId>,
        params: Self::Params,
    ) -> MaybeReady<Self::Result> {
        client
            .daemon
            .send(
                LspNotification {
                    method: Self::METHOD.into(),
                    params: serde_json::to_value(params).unwrap(),
                }
                .into(),
            )
            .expect("Failed to foward LSP notification to daemon");

        MaybeReady::Ready(NeverRespond)
    }
}
