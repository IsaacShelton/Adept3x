mod canonical;
mod file_content;
mod path_interner;

use crate::path_interner::{FileId, PathInterner};
pub use canonical::Canonical;
pub use file_content::FileContent;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Default)]
pub struct FileCache {
    path_interner: PathInterner,
    files: Mutex<HashMap<FileId, Arc<FileContent>>>,
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
