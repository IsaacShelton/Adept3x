use crate::repr::UnaliasedType;
use num_bigint::BigInt;
use ordered_float::NotNan;
use primitives::IntegerSign;

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
    Extend(IntegerSign),
    Truncate,
}

impl<'env> UnaryCast<'env> {
    pub fn is_dereference(&self) -> bool {
        matches!(self, Self::Dereference { .. })
    }
}

/*
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum BinOpMode {
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

impl BinOpMode {
    pub fn as_instr(&self, operands: ir::BinaryOperands) -> ir::Instr {
        match self {
            BinOpMode::Add(numeric_mode) => match numeric_mode {
                NumericMode::Integer(integer_sign) => todo!(),
                NumericMode::LooseIndeterminateSignInteger(cinteger) => todo!(),
                NumericMode::CheckOverflow(integer_bits, integer_sign) => todo!(),
                NumericMode::Float => todo!(),
            },
            BinOpMode::Subtract(numeric_mode) => todo!(),
            BinOpMode::Multiply(numeric_mode) => todo!(),
            BinOpMode::Divide(float_or_sign_lax) => todo!(),
            BinOpMode::Modulus(float_or_sign_lax) => todo!(),
            BinOpMode::Equals(float_or_integer) => todo!(),
            BinOpMode::NotEquals(float_or_integer) => todo!(),
            BinOpMode::LessThan(float_or_sign_lax) => todo!(),
            BinOpMode::LessThanEq(float_or_sign_lax) => todo!(),
            BinOpMode::GreaterThan(float_or_sign_lax) => todo!(),
            BinOpMode::GreaterThanEq(float_or_sign_lax) => todo!(),
            BinOpMode::BitwiseAnd => todo!(),
            BinOpMode::BitwiseOr => todo!(),
            BinOpMode::BitwiseXor => todo!(),
            BinOpMode::LeftShift => todo!(),
            BinOpMode::ArithmeticRightShift(sign_or_indeterminate) => todo!(),
            BinOpMode::LogicalLeftShift => todo!(),
            BinOpMode::LogicalRightShift => todo!(),
        }
    }
}
*/
