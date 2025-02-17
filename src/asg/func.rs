use crate::{asg::*, name::ResolvedName, source_files::Source, tag::Tag};
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct Func {
    pub name: ResolvedName,
    pub type_params: TypeParams,
    pub params: Params,
    pub return_type: Type,
    pub stmts: Vec<Stmt>,
    pub is_foreign: bool,
    pub is_generic: bool,
    pub vars: VariableStorage,
    pub source: Source,
    pub abide_abi: bool,
    pub tag: Option<Tag>,
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
    pub name: String,
    pub ty: Type,
}

impl Display for Param {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.name, self.ty)
    }
}

impl PartialEq for Param {
    fn eq(&self, other: &Self) -> bool {
        self.ty.eq(&other.ty)
    }
}
