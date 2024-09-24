use crate::{name::Name, source_files::Source};

#[derive(Clone, Debug)]
pub struct EnumMemberLiteral {
    pub enum_name: Name,
    pub variant_name: String,
    pub source: Source,
}
