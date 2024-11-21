use crate::{
    resolved::{EnumRef, HumanName},
    source_files::Source,
};
use core::hash::Hash;

#[derive(Clone, Debug)]
pub struct EnumMemberLiteral {
    pub human_name: HumanName,
    pub enum_ref: EnumRef,
    pub variant_name: String,
    pub source: Source,
}

impl Hash for EnumMemberLiteral {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.human_name.hash(state);
        self.enum_ref.hash(state);
        self.variant_name.hash(state);
    }
}

impl PartialEq for EnumMemberLiteral {
    fn eq(&self, other: &Self) -> bool {
        self.human_name.eq(&other.human_name)
            && self.enum_ref.eq(&other.enum_ref)
            && self.variant_name.eq(&other.variant_name)
    }
}

impl Eq for EnumMemberLiteral {}
