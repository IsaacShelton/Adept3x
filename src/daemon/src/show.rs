use lsp_message::{LspMessage, LspNotification, LspRequest, LspRequestId};
use lsp_types::{
    MessageActionItem, MessageType, ShowMessageParams, ShowMessageRequestParams,
    notification::Notification, request::Request,
};
use std::{collections::HashMap, io::Write};

#[allow(unused)]
pub fn show_message(stream: &mut impl Write, typ: MessageType, message: String) {
    let message = LspMessage::Notification(LspNotification {
        method: lsp_types::notification::ShowMessage::METHOD.into(),
        params: serde_json::to_value(ShowMessageParams { typ, message }).unwrap(),
    });

    let _ = message.write(stream);
}

#[allow(unused)]
pub fn show_message_request(
    stream: &mut impl std::io::Write,
    id: LspRequestId,
    typ: MessageType,
    message: String,
    options: impl Iterator<Item = String>,
) {
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

    let _ = message.write(stream);
}
