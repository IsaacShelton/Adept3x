use append_only_vec::AppendOnlyVec;
use std::{fs::read_to_string, process::exit};

#[derive(Debug)]
pub struct SourceFileCache {
    files: AppendOnlyVec<SourceFile>,
}

#[derive(Copy, Clone, Debug, PartialEq, Hash)]
pub struct SourceFileCacheKey {
    index: u32,
}

impl SourceFileCache {
    pub const INTERNAL_KEY: SourceFileCacheKey = SourceFileCacheKey { index: 0 };

    pub fn new() -> Self {
        let files = AppendOnlyVec::new();

        assert_eq!(
            files.push(SourceFile {
                filename: "<internal>".into(),
                content: "".into(),
            }),
            0
        );

        Self { files }
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

    pub fn add_or_exit(&self, filename: &str) -> SourceFileCacheKey {
        match self.add(&filename) {
            Ok(key) => key,
            Err(_) => {
                eprintln!("Failed to open file {}", filename);
                exit(1)
            }
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
