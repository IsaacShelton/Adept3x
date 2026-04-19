use crate::VfsFileContent;
use std::{path::Path, sync::Arc, time::SystemTime};

pub trait Fs {
    type IoError;
    fn last_modified(path: impl AsRef<Path>) -> Result<SystemTime, Self::IoError>;
    fn read(path: impl AsRef<Path>) -> Result<VfsFileContent, Self::IoError>;
}

pub struct BlockingFs;

impl Fs for BlockingFs {
    type IoError = std::io::Error;
    fn last_modified(path: impl AsRef<Path>) -> Result<SystemTime, Self::IoError> {
        Ok(std::fs::metadata(path)?
            .modified()
            .expect("failed to get last modified time for path"))
    }

    fn read(path: impl AsRef<Path>) -> Result<VfsFileContent, Self::IoError> {
        std::fs::read(path).map(|blob| VfsFileContent::new(Arc::from(blob)))
    }
}
