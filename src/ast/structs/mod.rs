use super::Type;
use crate::source_files::Source;
use derive_more::IsVariant;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct Struct {
    pub name: String,
    pub fields: IndexMap<String, Field>,
    pub parameters: IndexMap<String, TypeParameter>,
    pub is_packed: bool,
    pub source: Source,
    pub privacy: Privacy,
}

#[derive(Clone, Debug)]
pub struct TypeParameter {
    pub constraints: Vec<Type>,
}

impl TypeParameter {
    pub fn new(constraints: Vec<Type>) -> Self {
        Self { constraints }
    }
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
