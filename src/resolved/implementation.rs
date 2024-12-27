use super::{FunctionRef, Type};
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct Impl {
    pub resolved_type: Type,
    pub source: Source,
    pub body: Vec<FunctionRef>,
}
