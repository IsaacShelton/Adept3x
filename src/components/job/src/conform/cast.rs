use crate::repr::UnaliasedType;
use num_bigint::BigInt;
use ordered_float::NotNan;
use primitives::{FloatOrInteger, FloatOrSignLax, NumericMode, SignOrIndeterminate};

#[derive(Clone, Debug)]
pub enum UnaryCast<'env> {
    SpecializeBoolean(bool),
    SpecializeInteger(&'env BigInt),
    SpecializeFloat(Option<NotNan<f64>>),
    SpecializePointerOuter(UnaliasedType<'env>),
    SpecializeAsciiChar(u8),
    Dereference {
        after_deref: UnaliasedType<'env>,
        then: Option<&'env UnaryCast<'env>>,
    },
    ZeroExtend,
    SignExtend,
    Truncate,
}

impl<'env> UnaryCast<'env> {
    pub fn is_dereference(&self) -> bool {
        matches!(self, Self::Dereference { .. })
    }
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
