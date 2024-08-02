use super::Expr;
use crate::ast::{ConformBehavior, Type};

#[derive(Clone, Debug)]
pub struct StructureLiteral {
    pub ast_type: Type,
    pub fields: Vec<FieldInitializer>,
    pub fill_behavior: FillBehavior,
    pub conform_behavior: ConformBehavior,
}

#[derive(Clone, Debug)]
pub struct FieldInitializer {
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Copy, Clone, Debug)]
pub enum FillBehavior {
    Forbid,
    Zeroed,
}
