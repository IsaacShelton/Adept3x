use crate::{
    asg::{EnumRef, HumanName, Type},
    source_files::Source,
};
use core::hash::Hash;
use num::BigInt;

#[derive(Clone, Debug)]
pub struct EnumMemberLiteral {
    pub human_name: HumanName,
    pub enum_target: EnumTarget,
    pub variant_name: String,
    pub source: Source,
}

impl Hash for EnumMemberLiteral {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.human_name.hash(state);
        self.enum_target.hash(state);
        self.variant_name.hash(state);
    }
}

impl PartialEq for EnumMemberLiteral {
    fn eq(&self, other: &Self) -> bool {
        self.human_name.eq(&other.human_name)
            && self.enum_target.eq(&other.enum_target)
            && self.variant_name.eq(&other.variant_name)
    }
}

impl Eq for EnumMemberLiteral {}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EnumTarget {
    Named(EnumRef),
    Anonymous(BigInt, Type),
}
