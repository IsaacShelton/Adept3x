use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct EnumMemberLiteral {
    pub enum_name: String,
    pub variant_name: String,
    pub source: Source,
}
