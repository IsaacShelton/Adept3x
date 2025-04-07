use super::Type;
use attributes::Privacy;
use indexmap::IndexMap;
use num::BigInt;
use source_files::Source;

#[derive(Clone, Debug)]
pub struct Enum {
    pub name: String,
    pub backing_type: Option<Type>,
    pub source: Source,
    pub members: IndexMap<String, EnumMember>,
    pub privacy: Privacy,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EnumMember {
    pub value: BigInt,
    pub explicit_value: bool,
}
