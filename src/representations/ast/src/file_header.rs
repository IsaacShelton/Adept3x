use compiler_version::AdeptVersion;

#[derive(Clone, Debug, Default)]
pub struct FileHeader {
    pub adept: Option<AdeptVersion>,
}
