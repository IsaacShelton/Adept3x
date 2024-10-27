use super::Type;
use crate::{ast::Privacy, name::ResolvedName, source_files::Source};
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct Structure {
    pub name: ResolvedName,
    pub fields: IndexMap<String, Field>,
    pub is_packed: bool,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub resolved_type: Type,
    pub privacy: Privacy,
    pub source: Source,
}
