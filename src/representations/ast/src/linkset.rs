use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Linkset {
    pub entries: Vec<LinksetEntry>,
}

#[derive(Clone, Debug)]
pub enum LinksetEntry {
    File(PathBuf),
    Library(String),
    Framework(String),
}
