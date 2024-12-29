use super::{Parameters, Type};
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct Trait {
    pub source: Source,
    pub parameters: Vec<String>,
    pub functions: Vec<TraitFunction>,
}

#[derive(Clone, Debug)]
pub struct TraitFunction {
    pub name: String,
    pub parameters: Parameters,
    pub return_type: Type,
    pub source: Source,
}
