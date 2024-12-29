use super::{FuncRef, Type};
use crate::source_files::Source;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Impl {
    pub ty: Type,
    pub source: Source,
    pub body: HashMap<String, Vec<FuncRef>>,
}
