use super::Expr;
use crate::asg::Type;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct StructLiteral {
    pub structure_type: Type,
    pub fields: Vec<(String, Expr, usize)>,
}
