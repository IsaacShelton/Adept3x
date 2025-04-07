use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct SourceFile {
    filepath: PathBuf,
    content: String,
}

impl SourceFile {
    pub fn new(filename: PathBuf, content: String) -> Self {
        Self {
            filepath: filename,
            content,
        }
    }

    pub fn filename(&self) -> &str {
        self.filepath
            .to_str()
            .unwrap_or("<invalid unicode filename>")
    }

    pub fn filepath(&self) -> &Path {
        &self.filepath
    }

    pub fn content(&self) -> &str {
        &self.content
    }
}
