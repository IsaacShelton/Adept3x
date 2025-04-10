use crate::{ImplParams, Stmt, Type, TypeParams, VariableStorage, name::ResolvedName};
use attributes::{SymbolOwnership, Tag};
use source_files::Source;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct Func {
    pub name: ResolvedName,
    pub type_params: TypeParams,
    pub params: Params,
    pub return_type: Type,
    pub stmts: Vec<Stmt>,
    pub is_generic: bool,
    pub vars: VariableStorage,
    pub source: Source,
    pub abide_abi: bool,
    pub tag: Option<Tag>,
    pub ownership: SymbolOwnership,
    pub impl_params: ImplParams,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Params {
    pub required: Vec<Param>,
    pub is_cstyle_vararg: bool,
}

impl Display for Params {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (i, param) in self.required.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
        }

        if self.is_cstyle_vararg {
            if !self.required.is_empty() {
                write!(f, ", ")?;
            }

            write!(f, "...")?;
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Hash, Eq)]
pub struct Param {
    pub name: Option<String>,
    pub ty: Type,
}

impl Display for Param {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = &self.name {
            write!(f, "{} {}", name, self.ty)
        } else {
            write!(f, "{}", self.ty)
        }
    }
}

impl PartialEq for Param {
    fn eq(&self, other: &Self) -> bool {
        self.ty.eq(&other.ty)
    }
}
