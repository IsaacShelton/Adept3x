use derive_more::IsVariant;
use std::path::PathBuf;

#[derive(Clone, Debug, IsVariant)]
pub enum NormalFileKind {
    Adept,
    CSource,
    CHeader,
}

#[derive(Clone, Debug)]
pub struct NormalFile {
    pub kind: NormalFileKind,
    pub path: PathBuf,
}

impl NormalFile {
    pub fn adept(path: PathBuf) -> Self {
        Self {
            kind: NormalFileKind::Adept,
            path,
        }
    }

    pub fn c_source(path: PathBuf) -> Self {
        Self {
            kind: NormalFileKind::CSource,
            path,
        }
    }

    pub fn c_header(path: PathBuf) -> Self {
        Self {
            kind: NormalFileKind::CHeader,
            path,
        }
    }
}
