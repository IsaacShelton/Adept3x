use super::AstFile;
use crate::{file_id::FileId, source_files::SourceFiles};
use std::sync::atomic::{self, AtomicUsize};

#[derive(Debug)]
pub struct AstWorkspace<'a> {
    pub files: Vec<AstFile>,
    pub source_file_cache: &'a SourceFiles,
    pub next_file_id: AtomicUsize,
}

impl<'a> AstWorkspace<'a> {
    pub fn new(source_file_cache: &'a SourceFiles) -> Self {
        Self {
            files: Vec::new(),
            source_file_cache,
            next_file_id: AtomicUsize::new(0),
        }
    }

    pub fn new_file(&mut self) -> &mut AstFile {
        let file_id = FileId(self.next_file_id.fetch_add(1, atomic::Ordering::SeqCst));
        self.files.push(AstFile::new(file_id));
        self.files.last_mut().unwrap()
    }

    pub fn get(&self, id: FileId) -> Option<&AstFile> {
        self.files.get(id.0)
    }

    pub fn get_mut(&mut self, id: FileId) -> Option<&mut AstFile> {
        self.files.get_mut(id.0)
    }
}
