use crate::workspace::fs::{Fs, FsNodeId};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Name {
    pub namespace: Box<str>,
    pub basename: Box<str>,
}

impl Name {
    pub fn new(namespace: Option<impl Into<String>>, basename: impl Into<String>) -> Self {
        Self {
            namespace: namespace
                .map(|namespace| namespace.into())
                .unwrap_or_default()
                .into_boxed_str(),
            basename: basename.into().into_boxed_str(),
        }
    }

    pub fn plain(basename: impl Into<String>) -> Self {
        Self {
            namespace: "".into(),
            basename: basename.into().into_boxed_str(),
        }
    }

    pub fn into_plain(self) -> Option<String> {
        if self.namespace.is_empty() {
            Some(self.basename.to_string())
        } else {
            None
        }
    }

    pub fn as_plain_str(&self) -> Option<&str> {
        if self.namespace.is_empty() {
            Some(&self.basename)
        } else {
            None
        }
    }

    pub fn fullname(&self) -> String {
        if self.namespace.is_empty() {
            self.basename.clone().to_string()
        } else {
            format!("{}/{}", self.namespace, self.basename)
        }
    }
}

impl Display for Name {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.fullname())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResolvedName {
    fs_node_id: FsNodeId,
    name: Box<str>,
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
