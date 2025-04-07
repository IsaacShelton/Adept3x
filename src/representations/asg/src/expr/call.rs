use super::TypedExpr;
use crate::{FuncRef, PolyRecipe};

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
