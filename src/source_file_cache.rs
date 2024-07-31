use std::path::PathBuf;

use append_only_vec::AppendOnlyVec;

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

    pub fn add(&self, filename: PathBuf, content: String) -> SourceFileCacheKey {
        let index = self.files.push(SourceFile { filename, content });

        SourceFileCacheKey {
            index: index.try_into().unwrap(),
        }
    }
}

#[derive(Debug)]
pub struct SourceFile {
    filename: PathBuf,
    content: String,
}

impl SourceFile {
    pub fn filename(&self) -> &str {
        self.filename
            .to_str()
            .unwrap_or("<invalid unicode filename>")
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}
