use super::Expr;
use derive_more::IsVariant;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct UnaryMathOperation {
    pub operator: UnaryMathOperator,
    pub inner: Expr,
}

#[derive(Copy, Clone, Debug, Hash, PartialEq, Eq, IsVariant)]
pub enum UnaryMathOperator {
    Not,
    BitComplement,
    Negate,
    IsNonZero,
}

impl Display for UnaryMathOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Not => "!",
            Self::BitComplement => "~",
            Self::Negate => "-",
            Self::IsNonZero => "bool()",
        })
    }
}
