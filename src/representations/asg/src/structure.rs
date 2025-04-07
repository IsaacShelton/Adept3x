use super::{Type, TypeParams};
use crate::name::ResolvedName;
use attributes::Privacy;
use indexmap::IndexMap;
use source_files::Source;

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
