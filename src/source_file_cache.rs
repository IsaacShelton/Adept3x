use append_only_vec::AppendOnlyVec;
use std::{cell::RefCell, fs::read_to_string, pin::Pin};

#[derive(Debug)]
pub struct SourceFileCache {
    files: AppendOnlyVec<SourceFile>,
}

#[derive(Copy, Clone, Debug)]
pub struct SourceFileCacheKey {
    index: u32,
}

impl SourceFileCache {
    pub fn new() -> Self {
        Self {
            files: AppendOnlyVec::new(),
        }
    }

    pub fn get(&self, key: SourceFileCacheKey) -> &SourceFile {
        &self.files[key.index as usize]
    }

    pub fn add(&self, filename: &str) -> Result<SourceFileCacheKey, std::io::Error> {
        match read_to_string(filename) {
            Ok(content) => {
                let index = self.files.push(SourceFile {
                    filename: filename.to_string(),
                    content,
                });

                Ok(SourceFileCacheKey {
                    index: index.try_into().unwrap(),
                })
            }
            Err(error) => Err(error),
        }
    }
}

#[derive(Debug)]
pub struct SourceFile {
    filename: String,
    content: String,
}

impl SourceFile {
    pub fn filename(&self) -> &str {
        &self.filename
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}
