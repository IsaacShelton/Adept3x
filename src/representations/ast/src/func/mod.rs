mod params;

use super::{Given, Stmt, Type, TypeParams};
use crate::{UntypedCfg, flatten_func_ignore_const_evals};
use attributes::{Privacy, SymbolOwnership, Tag};
pub use params::{Param, Params};
use source_files::Source;

#[derive(Clone, Debug)]
pub struct Func {
    pub head: FuncHead,
    pub stmts: Vec<Stmt>,
    pub cfg: UntypedCfg,
}

impl Func {
    pub fn new(head: FuncHead, stmts: Vec<Stmt>) -> Self {
        let cfg = flatten_func_ignore_const_evals(&stmts, head.source);
        Self { head, stmts, cfg }
    }
}

#[derive(Clone, Debug)]
pub struct FuncHead {
    pub name: String,
    pub type_params: TypeParams,
    pub givens: Vec<Given>,
    pub params: Params,
    pub return_type: Type,
    pub source: Source,
    pub abide_abi: bool,
    pub tag: Option<Tag>,
    pub privacy: Privacy,
    pub ownership: SymbolOwnership,
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
