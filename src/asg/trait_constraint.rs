use super::{Parameters, Type};
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct Trait {
    pub source: Source,
    pub parameters: Vec<String>,
    pub funcs: Vec<TraitFunc>,
}

#[derive(Clone, Debug)]
pub struct TraitFunc {
    pub name: String,
    pub parameters: Parameters,
    pub return_type: Type,
    pub source: Source,
}
