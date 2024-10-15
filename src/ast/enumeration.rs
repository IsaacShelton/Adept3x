use super::{Privacy, Type};
use crate::source_files::Source;
use indexmap::IndexMap;
use num::BigInt;

#[derive(Clone, Debug)]
pub struct Enum {
    pub name: String,
    pub backing_type: Option<Type>,
    pub source: Source,
    pub members: IndexMap<String, EnumMember>,
    pub privacy: Privacy,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnumMember {
    pub value: BigInt,
    pub explicit_value: bool,
}
