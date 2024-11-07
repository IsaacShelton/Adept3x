use super::TypedExpr;
use crate::resolved::{FunctionRef, PolyValue};
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Call {
    pub function: FunctionRef,
    pub arguments: Vec<TypedExpr>,
}

#[derive(Clone, Debug)]
pub struct Callee {
    pub function: FunctionRef,
    pub recipe: HashMap<String, PolyValue>,
}
