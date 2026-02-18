mod completion;
mod did_change_text_document;
mod did_open_text_document;
mod document_diagnostic_request;
mod execute_command;
mod initialize;
mod initialized;
mod set_trace;
mod shutdown;

use crate::{LspEndpoint, MaybeReady};
use lsp_connection::LspConnection;
use lsp_message::{LspNotification, LspRequest, LspRequestId};

trait Forward: LspEndpoint {
    const IS_REQUEST: bool;
}

impl<T> LspEndpoint for T
where
    T: Forward,
{
    fn run(
        client: &mut LspConnection,
        id: Option<&LspRequestId>,
        params: Self::Params,
    ) -> MaybeReady<Self::Result> {
        let message = if Self::IS_REQUEST {
            LspRequest {
                id: id.unwrap().clone(),
                method: Self::METHOD.into(),
                params: serde_json::to_value(params).unwrap(),
            }
            .into()
        } else {
            LspNotification {
                method: Self::METHOD.into(),
                params: serde_json::to_value(params).unwrap(),
            }
            .into()
        };

        client
            .daemon
            .send(message)
            .expect("Failed to forward LSP message to daemon");
        MaybeReady::Pending
    }
}
