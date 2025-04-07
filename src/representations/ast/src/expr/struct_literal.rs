use crate::{Expr, Language, Type};

#[derive(Clone, Debug)]
pub struct StructLiteral {
    pub ast_type: Type,
    pub fields: Vec<FieldInitializer>,
    pub fill_behavior: FillBehavior,
    pub language: Language,
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
