use super::{Type, TypeParams};
use crate::{ast::Privacy, name::ResolvedName, source_files::Source};
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct Struct {
    pub name: ResolvedName,
    pub fields: IndexMap<String, Field>,
    pub is_packed: bool,
    pub params: TypeParams,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub ty: Type,
    pub privacy: Privacy,
    pub source: Source,
}
