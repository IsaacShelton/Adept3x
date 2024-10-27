use super::Expr;
use crate::resolved::Type;
use indexmap::IndexMap;

#[derive(Clone, Debug)]
pub struct StructLiteral {
    pub structure_type: Type,
    pub fields: IndexMap<String, (Expr, usize)>,
}
