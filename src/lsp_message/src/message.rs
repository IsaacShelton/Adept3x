use crate::{LspNotification, LspRequest, LspResponse};
use derive_more::From;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

#[derive(Clone, Debug, From, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LspMessage {
    Request(LspRequest),
    Response(LspResponse),
    Notification(LspNotification),
}

#[derive(Serialize)]
struct JsonRpc<'a> {
    jsonrpc: &'static str,
    #[serde(flatten)]
    msg: &'a LspMessage,
}

impl LspMessage {
    pub fn read(reader: &mut impl BufRead) -> io::Result<Option<LspMessage>> {
        let Some(text) = content_length::read(reader)? else {
            return Ok(None);
        };

        match serde_json::from_str(&text) {
            Ok(message) => Ok(Some(message)),
            Err(error) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Malformed LSP payload `{:?}`: {:?}", error, text),
            )),
        }
    }

    pub fn write(&self, writer: &mut impl Write) -> io::Result<()> {
        content_length::write(
            writer,
            &serde_json::to_string(&JsonRpc {
                jsonrpc: "2.0",
                msg: self,
            })?,
        )
    }
}
