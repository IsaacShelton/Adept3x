use crate::resolved::{
    FloatOrInteger, FloatOrSignLax, NumericMode, SignOrIndeterminate, TypedExpr,
};

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct BasicBinaryOperation {
    pub operator: BasicBinaryOperator,
    pub left: TypedExpr,
    pub right: TypedExpr,
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum BasicBinaryOperator {
    Add(NumericMode),
    Subtract(NumericMode),
    Multiply(NumericMode),
    Divide(FloatOrSignLax),
    Modulus(FloatOrSignLax),
    Equals(FloatOrInteger),
    NotEquals(FloatOrInteger),
    LessThan(FloatOrSignLax),
    LessThanEq(FloatOrSignLax),
    GreaterThan(FloatOrSignLax),
    GreaterThanEq(FloatOrSignLax),
    BitwiseAnd,
    BitwiseOr,
    BitwiseXor,
    LeftShift,
    ArithmeticRightShift(SignOrIndeterminate),
    LogicalLeftShift,
    LogicalRightShift,
}
