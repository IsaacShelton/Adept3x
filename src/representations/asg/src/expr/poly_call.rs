use super::TypedExpr;
use crate::PolyRecipe;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct PolyCall {
    pub callee: PolyCallee,
    pub args: Vec<TypedExpr>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct PolyCallee {
    pub polymorph: String,
    pub member: String,
    pub recipe: PolyRecipe,
}
