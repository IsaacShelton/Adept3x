use lsp_types::Uri;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
    str::FromStr,
};

pub trait EncodeFileUri {
    fn encode_file_uri(&self) -> Option<Uri>;
}

impl EncodeFileUri for Path {
    fn encode_file_uri(&self) -> Option<Uri> {
        let fragment = if !self.is_absolute() {
            Cow::from(strict_canonicalize(self)?)
        } else {
            Cow::from(self)
        };

        #[cfg(windows)]
        {
            Uri::from_str(&format!(
                "file:///{}",
                fragment.to_string_lossy().replace("\\", "/")
            ))
            .ok()
        }

        #[cfg(not(windows))]
        Uri::from_str(&format!("file://{}", fragment.to_string_lossy())).ok()
    }
}

#[inline]
#[cfg(not(windows))]
fn strict_canonicalize<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

/// On Windows, we should remove the wide path prefix `\\?` from `\\?\C:`
#[inline]
#[cfg(windows)]
fn strict_canonicalize<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
    let path = std::fs::canonicalize(path).ok()?;
    let head = path.components().next()?;

    let head = if let std::path::Component::Prefix(prefix) = head {
        if let std::path::Prefix::VerbatimDisk(disk) = prefix.kind() {
            Path::new(&format!("{}:", disk as char))
                .components()
                .next()?
        } else {
            head
        }
    } else {
        head
    };

    Some(
        std::iter::once(head)
            .chain(path.components().skip(1))
            .collect(),
    )
}
