use super::{Parameters, Privacy, Type};
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct Trait {
    pub name: String,
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
