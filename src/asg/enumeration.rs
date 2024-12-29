use super::Type;
use crate::{ast::EnumMember, name::ResolvedName, source_files::Source};
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct Enum {
    pub name: ResolvedName,
    pub ty: Type,
    pub source: Source,
    pub members: IndexMap<String, EnumMember>,
}
