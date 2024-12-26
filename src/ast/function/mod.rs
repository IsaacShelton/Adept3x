mod parameters;

use super::{Given, Privacy, Stmt, Type};
use crate::{source_files::Source, tag::Tag};
pub use parameters::{Parameter, Parameters};

#[derive(Clone, Debug)]
pub struct Function {
    pub head: FunctionHead,
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub struct FunctionHead {
    pub name: String,
    pub givens: Vec<Given>,
    pub parameters: Parameters,
    pub return_type: Type,
    pub is_foreign: bool,
    pub source: Source,
    pub abide_abi: bool,
    pub tag: Option<Tag>,
    pub privacy: Privacy,
}
