use super::{Function, Type};
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct Given {
    pub name: Option<String>,
    pub target: Type,
    pub source: Source,
    pub body: Vec<Function>,
}
