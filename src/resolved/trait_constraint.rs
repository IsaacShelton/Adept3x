use super::{Parameters, Type};
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct Trait {
    pub source: Source,
    pub methods: Vec<TraitMethod>,
}

#[derive(Clone, Debug)]
pub struct TraitMethod {
    pub name: String,
    pub parameters: Parameters,
    pub return_type: Type,
    pub source: Source,
}
