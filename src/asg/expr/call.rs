use super::TypedExpr;
use crate::{asg::FuncRef, resolve::PolyRecipe};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Call {
    pub callee: Callee,
    pub args: Vec<TypedExpr>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Callee {
    pub func_ref: FuncRef,
    pub recipe: PolyRecipe,
}
