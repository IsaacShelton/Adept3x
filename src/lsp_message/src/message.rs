use crate::{LspNotification, LspRequest, LspResponse};
use connection::Connection;
use derive_more::From;
use request::{BlockOn, CachedAft, PfIn};
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

#[derive(Clone, Debug, From, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LspMessage {
    Request(LspRequest),
    Response(LspResponse),
    Notification(LspNotification),
    ExtCompile(ExtCompile),
    ExtAft(ExtAft),
    ExtError(ExtError),
}

#[derive(Clone, Debug, From, Serialize, Deserialize)]
pub struct ExtAft {
    pub ext_aft: BlockOn<Option<CachedAft<PfIn>>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtCompile {
    pub ext_compile: String,
}

#[derive(Clone, Debug, From, Serialize, Deserialize)]
pub struct ExtError {
    pub ext_error: String,
}

#[derive(Serialize)]
struct JsonRpc<'a> {
    jsonrpc: &'static str,
    #[serde(flatten)]
    msg: &'a LspMessage,
}

impl LspMessage {
    pub fn send(connection: &Connection, message: LspMessage) -> io::Result<()> {
        connection
            .with_writer(|w| message.write_raw(w))
            .unwrap_or(Ok(()))
    }

    pub fn recv(connection: &Connection) -> io::Result<Option<LspMessage>> {
        connection
            .with_reader(|r| LspMessage::read_raw(r))
            .transpose()
            .map(|x| x.flatten())
    }

    pub fn read_raw(reader: &mut dyn BufRead) -> io::Result<Option<LspMessage>> {
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

    pub fn write_raw(&self, writer: impl Write) -> io::Result<()> {
        content_length::write(
            writer,
            &serde_json::to_string(&JsonRpc {
                jsonrpc: "2.0",
                msg: self,
            })?,
        )
    }
}
