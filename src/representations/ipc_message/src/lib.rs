use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use text_edit::{TextEdit, TextPosition};
use vfs::Canonical;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpcMessageId(pub usize);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Ipc {
    Request(IpcMessageId, IpcRequest),
    Response(IpcMessageId, IpcResponse),
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
    DidChange(IpcFile, Vec<TextEdit>),
    DidSave(IpcFile),
    Completion(TextPosition),
    Diagnostics(IpcFile),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum IpcResponse {
    Initialized,
    Changed,
    Saved,
    Completion(Vec<String>),
    Diagnostics(Vec<(String, TextPosition)>),
}
