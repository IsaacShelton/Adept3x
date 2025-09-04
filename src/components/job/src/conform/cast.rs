use crate::repr::UnaliasedType;
use num_bigint::BigInt;
use ordered_float::NotNan;
use primitives::{FloatOrInteger, FloatOrSignLax, NumericMode, SignOrIndeterminate};

#[derive(Clone, Debug)]
pub enum UnaryImplicitCast<'env> {
    SpecializeBoolean(bool),
    SpecializeInteger(&'env BigInt),
    SpecializeFloat(Option<NotNan<f64>>),
    SpecializePointerOuter(UnaliasedType<'env>),
    SpecializeAsciiChar(u8),
}

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum BinaryImplicitCast {
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
