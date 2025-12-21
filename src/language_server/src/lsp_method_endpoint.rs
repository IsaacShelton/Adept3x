use crate::{
    IntoLspResult, LspMethod, LspMethodHandled, MaybeReady, invalid_params, invalid_request_state,
};
use derive_more::From;
use lsp_connection::{LspConnection, LspMessage, LspResponse};

pub trait LspRequestEndpoint: LspMethod {
    fn run(
        client: &mut LspConnection,
        params: Self::Params,
    ) -> MaybeReady<<Self as LspMethod>::Result>;
}

#[derive(Clone, Debug, From)]
pub enum LspMethodDispatchResult {
    Handled(LspMethodHandled),
    NotHandled(LspMessage),
}

pub fn dispatch<Endpoint: LspRequestEndpoint>(
    client: &mut LspConnection,
    message: LspMessage,
) -> LspMethodDispatchResult {
    match message {
        LspMessage::Request(request) => {
            if request.method != Endpoint::METHOD {
                return LspMethodDispatchResult::NotHandled(request.into());
            }

            if Endpoint::REQUIRED_STATE.map_or(false, |state| state != client.connection_state) {
                log::error!(
                    "Invalid request '{}' message in LSP connection state {:?}",
                    request.method,
                    client.connection_state
                );

                return LspMethodHandled::ready(invalid_request_state(
                    request.id,
                    client.connection_state,
                ))
                .into();
            }

            let Ok(params) = serde_json::from_value(request.params) else {
                return LspMethodHandled::ready(invalid_params(request.id)).into();
            };

            match Endpoint::run(client, params) {
                MaybeReady::Ready(ready) => {
                    let result = ready.into_lsp_result().expect("LSP result for request");
                    let result = serde_json::to_value(result).expect("Can serialize response");

                    LspMethodHandled::ready(LspResponse {
                        id: request.id,
                        result: Some(result),
                        error: None,
                    })
                    .into()
                }
                MaybeReady::Pending => LspMethodHandled::pending().into(),
            }
        }
        LspMessage::Notification(notification) => {
            if notification.method != Endpoint::METHOD {
                return LspMethodDispatchResult::NotHandled(notification.into());
            }

            if Endpoint::REQUIRED_STATE.map_or(false, |state| state != client.connection_state) {
                return LspMethodHandled::WontRespond.into();
            }

            let Ok(params) = serde_json::from_value(notification.params) else {
                log::error!(
                    "Ignoring '{}' notification with invalid params",
                    &notification.method
                );
                return LspMethodHandled::WontRespond.into();
            };

            Endpoint::run(client, params);
            LspMethodHandled::WontRespond.into()
        }
        _ => return LspMethodDispatchResult::NotHandled(message),
    }
}
