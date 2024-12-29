use super::{FunctionRef, Type};
use crate::source_files::Source;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Impl {
    pub resolved_type: Type,
    pub source: Source,
    pub body: HashMap<String, Vec<FunctionRef>>,
}
