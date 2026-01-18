mod canonical;
mod file_bytes;
mod path_interner;

use crate::path_interner::{FileId, PathInterner};
pub use canonical::Canonical;
pub use file_bytes::FileBytes;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use text_edit::TextEditOrFullUtf16;

#[derive(Default)]
pub struct FileCache {
    path_interner: PathInterner,
    files: Mutex<HashMap<FileId, Arc<FileContent>>>,
}

#[derive(Copy, Clone, Debug)]
pub enum FileKind {
    Unknown,
    ProjectConfig,
    Adept,
}

#[derive(Debug)]
pub struct FileContent {
    pub kind: FileKind,
    pub file_bytes: FileBytes,
    pub syntax_tree: Option<()>,
}

impl FileContent {
    pub fn after_edits(&self, edits: impl Iterator<Item = TextEditOrFullUtf16>) -> Self {
        Self {
            kind: self.kind,
            file_bytes: self.file_bytes.after_edits(edits),
            syntax_tree: None,
        }
    }
}

impl FileCache {
    pub fn register_file(&self, relative_to: FileId, relative_path: &str) -> Option<FileId> {
        todo!("FileCache::register_file")
    }

    pub fn preregister_file(&mut self, filepath: Canonical<PathBuf>) -> FileId {
        self.path_interner.intern_mut(filepath)
    }

    pub fn get_content(&mut self, file_id: FileId) -> Option<Arc<FileContent>> {
        self.files.lock().unwrap().get(&file_id).cloned()
    }

    pub fn set_content(&mut self, file_id: FileId, file_content: FileContent) {
        self.files
            .lock()
            .unwrap()
            .insert(file_id, Arc::new(file_content));
    }
}
