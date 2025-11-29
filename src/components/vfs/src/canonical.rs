use derive_more::Deref;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Deref, Hash, PartialEq, Eq)]
pub struct Canonical<T>(T);

impl Canonical<PathBuf> {
    pub fn new(path: impl AsRef<Path>) -> Result<Self, ()> {
        Ok(Self(std::fs::canonicalize(path).map_err(|_| ())?))
    }
}

impl<P: AsRef<Path>> AsRef<Path> for Canonical<P> {
    fn as_ref(&self) -> &Path {
        self.0.as_ref()
    }
}
