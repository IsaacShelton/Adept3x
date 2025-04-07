use super::Type;
use crate::name::ResolvedName;
use ast::EnumMember;
use indexmap::IndexMap;
use source_files::Source;

#[derive(Clone, Debug)]
pub struct Enum {
    pub name: ResolvedName,
    pub ty: Type,
    pub source: Source,
    pub members: IndexMap<String, EnumMember>,
}
