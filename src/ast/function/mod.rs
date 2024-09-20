mod parameters;

use super::{Stmt, Type};
use crate::{source_files::Source, tag::Tag};
pub use parameters::{Parameter, Parameters};

#[derive(Clone, Debug)]
pub struct Function {
    pub name: String,
    pub parameters: Parameters,
    pub return_type: Type,
    pub stmts: Vec<Stmt>,
    pub is_foreign: bool,
    pub source: Source,
    pub abide_abi: bool,
    pub tag: Option<Tag>,
    pub namespace: Option<String>,
}
