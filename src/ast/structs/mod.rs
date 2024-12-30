use super::{Privacy, Type, TypeParams};
use crate::source_files::Source;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct Struct {
    pub name: String,
    pub params: TypeParams,
    pub fields: IndexMap<String, Field>,
    pub is_packed: bool,
    pub source: Source,
    pub privacy: Privacy,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub ast_type: Type,
    pub privacy: Privacy,
    pub source: Source,
}
