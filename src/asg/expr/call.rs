use super::TypedExpr;
use crate::{resolve::PolyRecipe, asg::FunctionRef};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Call {
    pub callee: Callee,
    pub arguments: Vec<TypedExpr>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Callee {
    pub function: FunctionRef,
    pub recipe: PolyRecipe,
}
