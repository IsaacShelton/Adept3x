use lsp_types::Uri;
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

pub trait DecodeFileUri {
    fn decode_file_uri<'a>(&'a self) -> Option<Cow<'a, Path>>;
}

impl DecodeFileUri for Uri {
    fn decode_file_uri<'a>(&'a self) -> Option<Cow<'a, Path>> {
        let path = match self.path().as_estr().decode().into_string_lossy() {
            Cow::Borrowed(path) => Cow::Borrowed(Path::new(path)),
            Cow::Owned(owned) => Cow::Owned(PathBuf::from(owned)),
        };

        #[cfg(windows)]
        {
            let authority = self.authority().expect("uri has no authority component");
            let host = authority.host().as_str();

            if host.is_empty() {
                // Very high chance this is a `file:///` uri, in which case the path
                // has a leading slash we need to remove.
                let host = path.to_string_lossy();
                return Some(Cow::Owned(PathBuf::from(&host[1..])));
            }

            Some(Cow::Owned(
                Path::new(&format!("{}:", host))
                    .components()
                    .chain(path.components())
                    .collect(),
            ))
        }

        #[cfg(not(windows))]
        Some(path)
    }
}
