use super::Expr;
use crate::{
    ast::{Type, TypeArg},
    name::Name,
};

#[derive(Clone, Debug)]
pub struct Call {
    pub name: Name,
    pub args: Vec<Expr>,
    pub expected_to_return: Option<Type>,
    pub generics: Vec<TypeArg>,
    pub using: Vec<Using>,
}

#[derive(Clone, Debug)]
pub struct Using {
    pub name: String,
    pub ty: Type,
}
