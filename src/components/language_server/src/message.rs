use ipc_message::GenericRequestId;
use serde_derive::{Deserialize, Serialize};
use smol::io::{AsyncBufRead, AsyncWrite};
use std::{
    io::{self, BufRead, Write},
    pin::Pin,
};
use transport::{
    read_message_raw_async, read_message_raw_sync, write_message_raw_async, write_message_raw_sync,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Message {
    Request(Request),
    Response(Response),
    Notification(Notification),
}

impl From<Request> for Message {
    fn from(request: Request) -> Message {
        Message::Request(request)
    }
}

impl From<Response> for Message {
    fn from(response: Response) -> Message {
        Message::Response(response)
    }
}

impl From<Notification> for Message {
    fn from(notification: Notification) -> Message {
        Message::Notification(notification)
    }
}

pub type RequestId = GenericRequestId;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Request {
    pub id: RequestId,
    pub method: String,
    #[serde(default = "serde_json::Value::default")]
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub params: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Response {
    pub id: RequestId,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<ResponseError>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Notification {
    pub method: String,
    #[serde(default = "serde_json::Value::default")]
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub params: serde_json::Value,
}

#[derive(Serialize)]
struct JsonRpc<'a> {
    jsonrpc: &'static str,
    #[serde(flatten)]
    msg: &'a Message,
}

impl Message {
    pub fn read_sync(reader: &mut impl BufRead) -> io::Result<Option<Message>> {
        let Some(text) = read_message_raw_sync(reader)? else {
            return Ok(None);
        };

        let msg = match serde_json::from_str(&text) {
            Ok(msg) => msg,
            Err(error) => {
                return Err(malformed(format!(
                    "Malformed LSP payload `{:?}`: {:?}",
                    error, text
                )));
            }
        };

        Ok(Some(msg))
    }

    pub async fn read_async(reader: Pin<&mut impl AsyncBufRead>) -> io::Result<Option<Message>> {
        let Some(text) = read_message_raw_async(reader).await? else {
            return Ok(None);
        };

        let msg = match serde_json::from_str(&text) {
            Ok(msg) => msg,
            Err(error) => {
                return Err(malformed(format!(
                    "Malformed LSP payload `{:?}`: {:?}",
                    error, text
                )));
            }
        };

        Ok(Some(msg))
    }

    pub fn write_sync(&self, writer: &mut impl Write) -> io::Result<()> {
        let text = serde_json::to_string(&JsonRpc {
            jsonrpc: "2.0",
            msg: self,
        })?;

        write_message_raw_sync(writer, &text)
    }

    pub async fn write_async(&self, writer: Pin<&mut impl AsyncWrite>) -> io::Result<()> {
        let text = serde_json::to_string(&JsonRpc {
            jsonrpc: "2.0",
            msg: self,
        })?;

        write_message_raw_async(writer, &text).await
    }
}

fn malformed(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> io::Error {
    io::Error::new(smol::io::ErrorKind::InvalidData, error)
}
