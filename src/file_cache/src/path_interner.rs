use crate::Canonical;
use std::{borrow::Cow, collections::HashMap, path::PathBuf, sync::Mutex};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct FileId(usize);

#[derive(Default)]
pub struct PathInterner {
    inner: Mutex<PathInternerInner>,
}

#[derive(Default)]
pub struct PathInternerInner {
    paths: HashMap<FileId, Canonical<PathBuf>>,
    file_ids: HashMap<Canonical<PathBuf>, FileId>,
    next_file_id: usize,
}

impl PathInterner {
    pub fn intern(&self, filepath: Cow<Canonical<PathBuf>>) -> FileId {
        self.inner.lock().unwrap().intern(filepath)
    }

    pub fn intern_mut(&mut self, filepath: Cow<Canonical<PathBuf>>) -> FileId {
        self.inner.get_mut().unwrap().intern(filepath)
    }
}

impl PathInternerInner {
    pub fn intern(&mut self, filepath: Cow<Canonical<PathBuf>>) -> FileId {
        if let Some(found) = self.file_ids.get(&filepath) {
            return *found;
        }

        let file_id = FileId(self.next_file_id);
        self.next_file_id += 1;

        self.paths.insert(file_id, filepath.as_ref().clone());
        self.file_ids.insert(filepath.into_owned(), file_id);
        file_id
    }
}
