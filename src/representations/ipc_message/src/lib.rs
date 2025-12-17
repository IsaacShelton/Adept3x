use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    path::PathBuf,
    sync::Arc,
};
use text_edit::{TextEdit, TextPosition};
use vfs::Canonical;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcMessageId(pub usize);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IpcMessage {
    Request(Option<IpcMessageId>, Option<GenericRequestId>, IpcRequest),
    Response(Option<IpcMessageId>, Option<GenericRequestId>, IpcResponse),
    Notification(Option<GenericRequestId>, IpcNotification),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IpcFile {
    filename: Canonical<PathBuf>,
}

impl IpcFile {
    pub fn new(filename: Canonical<PathBuf>) -> Self {
        Self { filename }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IpcRequest {
    Initialize { fingerprint: String },
    Shutdown,
    Completion(TextPosition),
    Diagnostics(IpcFile),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IpcResponse {
    Initialized,
    ShuttingDown,
    Changed,
    Saved,
    Completion(Vec<String>),
    Diagnostics(Vec<(String, TextPosition)>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IpcNotification {
    Exit,
    DidOpen(IpcFile),
    DidChange(IpcFile, Vec<TextEdit>),
    DidSave(IpcFile),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct GenericRequestId(GenericRequestIdRepr);

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(untagged)]
enum GenericRequestIdRepr {
    Int(i32),
    String(Arc<str>),
}

impl From<i32> for GenericRequestId {
    fn from(id: i32) -> GenericRequestId {
        GenericRequestId(GenericRequestIdRepr::Int(id))
    }
}

impl From<String> for GenericRequestId {
    fn from(id: String) -> GenericRequestId {
        GenericRequestId(GenericRequestIdRepr::String(Arc::from(id)))
    }
}

impl Display for GenericRequestId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            GenericRequestIdRepr::Int(it) => write!(f, "{}", it),
            GenericRequestIdRepr::String(it) => write!(f, "{:?}", it),
        }
    }
}
