use super::TypedExpr;
use crate::{asg::FuncRef, resolve::PolyRecipe};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Call {
    pub callee: Callee,
    pub arguments: Vec<TypedExpr>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Callee {
    pub function: FuncRef,
    pub recipe: PolyRecipe,
}
