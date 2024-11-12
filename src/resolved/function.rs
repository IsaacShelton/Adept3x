use crate::{name::ResolvedName, resolved::*, source_files::Source, tag::Tag};
use std::{collections::HashSet, fmt::Display};

#[derive(Clone, Debug)]
pub struct Function {
    pub name: ResolvedName,
    pub parameters: Parameters,
    pub return_type: Type,
    pub stmts: Vec<Stmt>,
    pub is_foreign: bool,
    pub is_generic: bool,
    pub variables: VariableStorage,
    pub source: Source,
    pub abide_abi: bool,
    pub tag: Option<Tag>,
    pub constraints: HashSet<Constraint>,
}

#[derive(Clone, Debug, Default)]
pub struct Parameters {
    pub required: Vec<Parameter>,
    pub is_cstyle_vararg: bool,
}

impl Display for Parameters {
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

#[derive(Clone, Debug)]
pub struct Parameter {
    pub name: String,
    pub resolved_type: Type,
}

impl Display for Parameter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.name, self.resolved_type)
    }
}

impl PartialEq for Parameter {
    fn eq(&self, other: &Self) -> bool {
        self.resolved_type.eq(&other.resolved_type)
    }
}
