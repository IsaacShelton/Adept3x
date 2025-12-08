mod canonical;
mod file;
mod fs;
mod view;

pub use canonical::*;
use derive_more::Deref;
pub use file::*;
pub use fs::*;
use idle::IdleTracker;
use std::{
    collections::HashMap,
    fmt::Debug,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::SystemTime,
};

pub struct Vfs {
    files: Mutex<HashMap<Arc<Canonical<PathBuf>>, VfsFile>>,
    idle_tracker: Option<Arc<IdleTracker>>,
}

impl Vfs {
    pub fn new(idle_tracker: Option<Arc<IdleTracker>>) -> Self {
        Self {
            files: Default::default(),
            idle_tracker,
        }
    }
}

impl Debug for Vfs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vfs").finish()
    }
}

#[derive(Clone, Debug, Deref)]
pub struct DidChange<T> {
    #[deref]
    pub content: T,
    pub changed_at: Option<SystemTime>,
}

impl<T> DidChange<T> {
    pub fn new_changed(content: T, at: SystemTime) -> Self {
        Self {
            content,
            changed_at: Some(at),
        }
    }

    pub fn new_unchanged(content: T) -> Self {
        Self {
            content,
            changed_at: None,
        }
    }
}

impl Vfs {
    pub fn read<FS: Fs>(
        &self,
        filename: Arc<Canonical<PathBuf>>,
    ) -> Result<DidChange<VfsFileContent>, FS::IoError> {
        let mut files = self.files.lock().unwrap();

        let got = if let Some(file) = files.get_mut(&filename) {
            let new_last_modified = FS::last_modified(&**filename)?;

            if file.last_modified != new_last_modified {
                DidChange::new_changed(FS::read(&**filename)?, new_last_modified)
            } else {
                DidChange::new_unchanged(file.content.clone())
            }
        } else {
            let last_modified = FS::last_modified(&**filename)?;
            DidChange::new_changed(FS::read(&**filename)?, last_modified)
        };

        if let Some(last_modified) = got.changed_at {
            if let Some(idle_tracker) = &self.idle_tracker {
                idle_tracker.still_active();
            }

            files.entry(filename).insert_entry(VfsFile {
                is_buffer: false,
                content: got.content.clone(),
                last_modified,
            });
        }

        Ok(got)
    }
}
