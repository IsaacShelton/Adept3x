use crate::{
    Handled, IntoLspResult, LspEndpoint, MaybeReady, invalid_params, invalid_request_state,
};
use derive_more::From;
use lsp_connection::LspConnection;
use lsp_message::{LspMessage, LspResponse};

#[derive(Clone, Debug, From)]
pub enum DispatchResult {
    Handled(Handled),
    NotHandled(LspMessage),
}

pub fn dispatch<Endpoint: LspEndpoint>(
    client: &mut LspConnection,
    message: LspMessage,
) -> DispatchResult {
    match message {
        LspMessage::Request(request) => {
            if request.method != Endpoint::METHOD {
                return DispatchResult::NotHandled(request.into());
            }

            if client.connection_state != Endpoint::REQUIRED_CONNECTION_STATE {
                log::error!(
                    "Invalid request '{}' message in LSP connection state {:?}",
                    request.method,
                    client.connection_state
                );
                return Handled::ready(invalid_request_state(request.id, client.connection_state))
                    .into();
            }

            let Ok(params) = serde_json::from_value(request.params) else {
                return Handled::ready(invalid_params(request.id)).into();
            };

            match Endpoint::run(client, Some(&request.id), params) {
                MaybeReady::Ready(ready) => Handled::ready(LspResponse {
                    id: request.id,
                    result: Some(
                        serde_json::to_value(ready.into_lsp_result().expect("has response"))
                            .expect("can serialize response"),
                    ),
                    error: None,
                })
                .into(),
                MaybeReady::Pending => Handled::pending().into(),
            }
        }
        LspMessage::Notification(notification) => {
            if notification.method != Endpoint::METHOD {
                return DispatchResult::NotHandled(notification.into());
            }

            if client.connection_state != Endpoint::REQUIRED_CONNECTION_STATE {
                log::error!(
                    "Ignoring '{}' notification, we are not in the right state to accept it",
                    &notification.method
                );
                return Handled::WontRespond.into();
            }

            let Ok(params) = serde_json::from_value(notification.params) else {
                log::error!(
                    "Ignoring '{}' notification with invalid params",
                    &notification.method
                );
                return Handled::WontRespond.into();
            };

            Endpoint::run(client, None, params);
            Handled::WontRespond.into()
        }
        _ => return DispatchResult::NotHandled(message),
    }
}
