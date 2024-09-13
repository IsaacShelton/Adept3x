use super::Expr;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct UnaryOperation {
    pub operator: UnaryOperator,
    pub inner: Expr,
}

#[derive(Clone, Debug)]
pub enum UnaryOperator {
    Not,
    BitComplement,
    Negate,
    IsNonZero,
    AddressOf,
    Dereference,
}

impl Display for UnaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Not => "!",
            Self::BitComplement => "~",
            Self::Negate => "-",
            Self::IsNonZero => "bool()",
            Self::AddressOf => "(address of) &",
            Self::Dereference => "(dereference) *",
        })
    }
}
