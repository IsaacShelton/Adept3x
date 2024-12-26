mod parameters;

use super::{Given, Privacy, Stmt, Type};
use crate::{name::Name, source_files::Source, tag::Tag};
pub use parameters::{Parameter, Parameters};

#[derive(Clone, Debug)]
pub struct Function {
    pub name: Name,
    pub givens: Vec<Given>,
    pub parameters: Parameters,
    pub return_type: Type,
    pub stmts: Vec<Stmt>,
    pub is_foreign: bool,
    pub source: Source,
    pub abide_abi: bool,
    pub tag: Option<Tag>,
    pub privacy: Privacy,
}
