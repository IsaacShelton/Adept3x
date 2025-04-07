use ast::Name;
use fs_tree::{Fs, FsNodeId};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResolvedName {
    pub fs_node_id: FsNodeId,
    pub name: Box<str>,
}

impl ResolvedName {
    pub fn new(fs_node_id: FsNodeId, name: &Name) -> Self {
        Self {
            fs_node_id,
            name: name.fullname().into_boxed_str(),
        }
    }

    pub fn plain(&self) -> &str {
        &*self.name
    }

    pub fn display<'a>(&'a self, fs: &'a Fs) -> DisplayResolvedName<'a> {
        DisplayResolvedName { name: self, fs }
    }
}

pub struct DisplayResolvedName<'a> {
    name: &'a ResolvedName,
    fs: &'a Fs,
}

impl Display for DisplayResolvedName<'_> {
    #[allow(dead_code)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let filename = &self.fs.get(self.name.fs_node_id).filename;
        let prefix = if cfg!(target_os = "windows") { "" } else { "/" };

        write!(
            f,
            "{}{} - {}",
            prefix,
            filename.to_string_lossy(),
            self.name.plain(),
        )?;

        Ok(())
    }
}
