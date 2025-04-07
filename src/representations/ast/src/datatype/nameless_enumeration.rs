use super::Type;
use crate::EnumMember;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct AnonymousEnum {
    pub members: IndexMap<String, EnumMember>,
    pub backing_type: Option<Box<Type>>,
    pub allow_implicit_integer_conversions: bool,
}
