use super::{Function, Type};
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct Impl {
    pub for_type: Type,
    pub target_trait: Type,
    pub source: Source,
    pub body: Vec<Function>,
}
