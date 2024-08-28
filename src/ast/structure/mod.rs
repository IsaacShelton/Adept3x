use super::Type;
use crate::source_files::Source;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct Structure {
    pub name: String,
    pub fields: IndexMap<String, Field>,
    pub is_packed: bool,
    pub source: Source,
}

#[derive(Copy, Clone, Debug, Default)]
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
