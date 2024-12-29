use super::{Func, Type};
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct Impl {
    pub name: Option<String>,
    pub target: Type,
    pub source: Source,
    pub body: Vec<Func>,
}
