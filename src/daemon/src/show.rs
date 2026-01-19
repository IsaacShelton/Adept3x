#[cfg(target_family = "unix")]
use lsp_message::LspRequestId;
use lsp_message::{LspMessage, LspNotification};
#[cfg(target_family = "unix")]
use lsp_types::{MessageType, ShowMessageParams, notification::Notification};
#[cfg(target_family = "unix")]
use std::os::unix::net::UnixStream;

#[cfg(target_family = "unix")]
pub fn show_message(mut stream: &UnixStream, typ: MessageType, message: String) {
    let message = LspMessage::Notification(LspNotification {
        method: lsp_types::notification::ShowMessage::METHOD.into(),
        params: serde_json::to_value(ShowMessageParams { typ, message }).unwrap(),
    });

    let _ = message.write(&mut stream);
}

#[cfg(target_family = "unix")]
pub fn show_message_request(
    mut stream: &UnixStream,
    id: LspRequestId,
    typ: MessageType,
    message: String,
    options: impl Iterator<Item = String>,
) {
    use lsp_message::LspRequest;
    use lsp_types::{MessageActionItem, ShowMessageRequestParams, request::Request};
    use std::collections::HashMap;

    let message = LspMessage::Request(LspRequest {
        id,
        method: lsp_types::request::ShowMessageRequest::METHOD.into(),
        params: serde_json::to_value(ShowMessageRequestParams {
            typ,
            message,
            actions: Some(
                options
                    .map(|option| MessageActionItem {
                        title: option,
                        properties: HashMap::default(),
                    })
                    .collect(),
            ),
        })
        .unwrap(),
    });

    let _ = message.write(&mut stream);
}
