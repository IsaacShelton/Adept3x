use super::{Type, TypeParams};
use attributes::Privacy;
use indexmap::IndexMap;
use source_files::Source;

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
