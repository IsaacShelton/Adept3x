use crate::{Language, expr::Expr};
use derive_more::IsVariant;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub struct BasicBinaryOperation {
    pub operator: BasicBinaryOperator,
    pub left: Expr,
    pub right: Expr,
    pub language: Language,
}

#[derive(Copy, Clone, Debug, IsVariant)]
pub enum BasicBinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,
    Equals,
    NotEquals,
    LessThan,
    LessThanEq,
    GreaterThan,
    GreaterThanEq,
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    RightShift,
    LogicalLeftShift,
    LogicalRightShift,
}

impl BasicBinaryOperator {
    pub fn verb(&self) -> &'static str {
        match self {
            BasicBinaryOperator::Add => "add",
            BasicBinaryOperator::Subtract => "subtract",
            BasicBinaryOperator::Multiply => "multiply",
            BasicBinaryOperator::Divide => "divide",
            BasicBinaryOperator::Modulus => "modulo",
            BasicBinaryOperator::Equals => "compare",
            BasicBinaryOperator::NotEquals => "compare",
            BasicBinaryOperator::LessThan => "compare",
            BasicBinaryOperator::LessThanEq => "compare",
            BasicBinaryOperator::GreaterThan => "compare",
            BasicBinaryOperator::GreaterThanEq => "compare",
            BasicBinaryOperator::BitwiseAnd => "bitwise-and",
            BasicBinaryOperator::BitwiseOr => "bitwise-or",
            BasicBinaryOperator::BitwiseXor => "bitwise-xor",
            BasicBinaryOperator::LeftShift => "left-shift",
            BasicBinaryOperator::RightShift => "right-shift",
            BasicBinaryOperator::LogicalLeftShift => "logical-left-shift",
            BasicBinaryOperator::LogicalRightShift => "logical-right-shift",
        }
    }

    pub fn returns_boolean(&self) -> bool {
        match self {
            Self::Equals
            | Self::NotEquals
            | Self::LessThan
            | Self::LessThanEq
            | Self::GreaterThan
            | Self::GreaterThanEq => true,
            Self::Add
            | Self::Subtract
            | Self::Multiply
            | Self::Divide
            | Self::Modulus
            | Self::BitwiseAnd
            | Self::BitwiseOr
            | Self::BitwiseXor
            | Self::LeftShift
            | Self::RightShift
            | Self::LogicalLeftShift
            | Self::LogicalRightShift => false,
        }
    }
}

impl Display for BasicBinaryOperator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Modulus => "%",
            Self::Equals => "==",
            Self::NotEquals => "!=",
            Self::LessThan => "<",
            Self::LessThanEq => "<=",
            Self::GreaterThan => ">",
            Self::GreaterThanEq => ">=",
            Self::BitwiseAnd => "&",
            Self::BitwiseOr => "|",
            Self::BitwiseXor => "^",
            Self::LeftShift => "<<",
            Self::RightShift => ">>",
            Self::LogicalLeftShift => "<<<",
            Self::LogicalRightShift => ">>>",
        })
    }
}
