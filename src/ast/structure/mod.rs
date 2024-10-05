use super::Type;
use crate::{name::Name, source_files::Source};
use derive_more::IsVariant;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct Structure {
    pub name: Name,
    pub fields: IndexMap<String, Field>,
    pub is_packed: bool,
    pub source: Source,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, IsVariant)]
pub enum Privacy {
    #[default]
    Public,
    Private,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub ast_type: Type,
    pub privacy: Privacy,
    pub source: Source,
}
