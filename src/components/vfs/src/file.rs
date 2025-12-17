use std::{sync::Arc, time::SystemTime};
use text_edit::TextEdit;

pub struct VfsFile {
    pub is_buffer: bool,
    pub content: VfsFileContent,
    pub last_modified: SystemTime,
}

#[derive(Clone, Debug)]
pub struct VfsFileContent {
    content: Arc<[u8]>,
    is_text: bool,
}

impl VfsFileContent {
    pub fn new(content: Arc<[u8]>) -> Self {
        let is_text = str::from_utf8(&content).is_ok();
        Self { content, is_text }
    }

    pub fn text(&self) -> Result<Arc<str>, ()> {
        if self.is_text {
            let raw = Arc::into_raw(self.content.clone());
            Ok(unsafe { Arc::from_raw(raw as *const str) })
        } else {
            Err(())
        }
    }
}

#[derive(Clone, Debug)]
pub struct IncrementalVfsFileContent {
    pub rest_content: Arc<[u8]>,
    pub edits: Option<Arc<[TextEdit]>>,
    pub is_text: bool,
}
