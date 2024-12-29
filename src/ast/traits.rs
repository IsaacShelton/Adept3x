use super::{Parameters, Privacy, Type};
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct Trait {
    pub name: String,
    pub parameters: Vec<String>,
    pub source: Source,
    pub privacy: Privacy,
    pub functions: Vec<TraitFunction>,
}

#[derive(Clone, Debug)]
pub struct TraitFunction {
    pub name: String,
    pub parameters: Parameters,
    pub return_type: Type,
    pub source: Source,
}
