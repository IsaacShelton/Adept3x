use super::Expr;
use crate::{NamePath, Type, TypeArg};
use source_files::Sourced;

#[derive(Clone, Debug)]
pub struct Call {
    pub name_path: NamePath,
    pub args: Vec<Expr>,
    pub expected_to_return: Option<Type>,
    pub generics: Vec<TypeArg>,
    pub using: Vec<Using>,
}

#[derive(Clone, Debug)]
pub struct Using {
    pub name: Option<Sourced<String>>,
    pub ty: Type,
}
