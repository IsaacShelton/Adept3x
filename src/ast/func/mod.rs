mod params;

use super::{Given, Privacy, Stmt, Type, TypeParams};
use crate::{source_files::Source, tag::Tag};
pub use params::{Param, Params};

#[derive(Clone, Debug)]
pub struct Func {
    pub head: FuncHead,
    pub stmts: Vec<Stmt>,
}

#[derive(Clone, Debug)]
pub struct FuncHead {
    pub name: String,
    pub type_params: TypeParams,
    pub givens: Vec<Given>,
    pub params: Params,
    pub return_type: Type,
    pub is_foreign: bool,
    pub is_exposed: bool,
    pub source: Source,
    pub abide_abi: bool,
    pub tag: Option<Tag>,
    pub privacy: Privacy,
}

impl FuncHead {
    pub fn is_generic(&self) -> bool {
        self.return_type.contains_polymorph().is_some()
            || self
                .params
                .required
                .iter()
                .any(|param| param.ast_type.contains_polymorph().is_some())
            || !self.type_params.is_empty()
            || !self.givens.is_empty()
    }
}
