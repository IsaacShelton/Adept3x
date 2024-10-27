use crate::{
    resolved::{EnumRef, HumanName},
    source_files::Source,
};

#[derive(Clone, Debug)]
pub struct EnumMemberLiteral {
    pub human_name: HumanName,
    pub enum_ref: EnumRef,
    pub variant_name: String,
    pub source: Source,
}
