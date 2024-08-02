use std::{ffi::OsStr, path::Path};

pub fn normalized_path_segments(path: &Path) -> Vec<&OsStr> {
    let mut total = Vec::new();

    for segment in path.components() {
        use std::path::Component::*;

        match segment {
            Prefix(p) => total.push(p.as_os_str()),
            RootDir => total.clear(),
            CurDir => {}
            ParentDir => {
                total.pop();
            }
            Normal(n) => total.push(n),
        }
    }

    total
}
