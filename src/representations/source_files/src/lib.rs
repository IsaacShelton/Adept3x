mod file;
mod key;
mod source;

use append_only_vec::AppendOnlyVec;
pub use file::SourceFile;
pub use key::SourceFileKey;
pub use source::{Source, Sourced};
use std::path::PathBuf;

#[derive(Debug)]
pub struct SourceFiles {
    files: AppendOnlyVec<SourceFile>,
}

impl SourceFiles {
    pub const INTERNAL_KEY: SourceFileKey = SourceFileKey(0);

    pub fn new() -> Self {
        let files = AppendOnlyVec::new();

        // Create the <internal> file, used for code created by the compiler itself
        assert_eq!(
            files.push(SourceFile::new("<internal>".into(), "".into())),
            Self::INTERNAL_KEY.0.try_into().unwrap(),
        );

        Self { files }
    }

    pub fn get(&self, key: SourceFileKey) -> &SourceFile {
        &self.files[key.0 as usize]
    }

    pub fn add(&self, filename: PathBuf, content: String) -> SourceFileKey {
        SourceFileKey(
            self.files
                .push(SourceFile::new(filename, content))
                .try_into()
                .unwrap(),
        )
    }
}
