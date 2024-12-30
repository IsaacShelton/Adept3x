use super::Expr;
use crate::{
    ast::{CompileTimeArgument, Type},
    name::Name,
};

#[derive(Clone, Debug)]
pub struct Call {
    pub name: Name,
    pub args: Vec<Expr>,
    pub expected_to_return: Option<Type>,
    pub generics: Vec<CompileTimeArgument>,
}
