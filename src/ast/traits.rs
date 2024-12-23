use super::{Parameters, Privacy, Type, TypeParameter};
use crate::source_files::Source;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct Trait {
    pub name: String,
    pub parameters: IndexMap<String, TypeParameter>,
    pub source: Source,
    pub privacy: Privacy,
    pub methods: Vec<TraitMethod>,
}

#[derive(Clone, Debug)]
pub struct TraitMethod {
    pub name: String,
    pub parameters: Parameters,
    pub return_type: Type,
    pub source: Source,
}
