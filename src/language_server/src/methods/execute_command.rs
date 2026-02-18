use crate::{LspEndpoint, LspMethod, MaybeReady, Static};
use lsp_connection::LspConnection;
use lsp_message::{LspRequest, LspRequestId};

impl LspEndpoint for Static<lsp_types::request::ExecuteCommand> {
    fn run(
        client: &mut LspConnection,
        id: Option<&LspRequestId>,
        params: Self::Params,
    ) -> MaybeReady<Self::Result> {
        let id = id.unwrap();

        client
            .daemon
            .send(
                LspRequest {
                    id: id.clone(),
                    method: Self::METHOD.into(),
                    params: serde_json::to_value(params).unwrap(),
                }
                .into(),
            )
            .expect("Failed to foward LSP notification to daemon");

        MaybeReady::Pending
    }
}
