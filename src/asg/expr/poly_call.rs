use super::TypedExpr;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct PolyCall {
    pub callee: PolyCallee,
    pub arguments: Vec<TypedExpr>,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct PolyCallee {
    pub polymorph: String,
    pub member: String,
}
