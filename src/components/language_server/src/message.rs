use serde_derive::{Deserialize, Serialize};
use smol::io::{AsyncBufRead, AsyncWrite};
use std::{fmt, pin::Pin};
use transport::{read_message_raw, write_message_raw};

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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct RequestId(RequestIdRepr);

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(untagged)]
enum RequestIdRepr {
    Int(i32),
    String(String),
}

impl From<i32> for RequestId {
    fn from(id: i32) -> RequestId {
        RequestId(RequestIdRepr::Int(id))
    }
}

impl From<String> for RequestId {
    fn from(id: String) -> RequestId {
        RequestId(RequestIdRepr::String(id))
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.0 {
            RequestIdRepr::Int(it) => write!(f, "{}", it),
            RequestIdRepr::String(it) => write!(f, "{:?}", it),
        }
    }
}

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
    pub async fn read(reader: Pin<&mut impl AsyncBufRead>) -> smol::io::Result<Option<Message>> {
        let Some(text) = read_message_raw(reader).await? else {
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

    pub async fn write(&self, writer: Pin<&mut impl AsyncWrite>) -> smol::io::Result<()> {
        let text = serde_json::to_string(&JsonRpc {
            jsonrpc: "2.0",
            msg: self,
        })?;

        write_message_raw(writer, &text).await
    }
}

fn malformed(error: impl Into<Box<dyn std::error::Error + Send + Sync>>) -> smol::io::Error {
    smol::io::Error::new(smol::io::ErrorKind::InvalidData, error)
}
