/*
    ==================  components/job/src/typed_cfg/mod.rs  ==================
    Contains definitions for typing and resolving references for a CFG
    ---------------------------------------------------------------------------
*/

mod value;

use crate::repr::UnaliasedType;
use derive_more::From;
use num_bigint::BigInt;
use ordered_float::NotNan;
use primitives::{FloatOrInteger, FloatOrSignLax, NumericMode, SignOrIndeterminate};
pub use value::*;

#[derive(Clone, Debug)]
pub struct Resolved<'env> {
    ty: UnaliasedType<'env>,
    #[allow(unused)]
    data: Option<ResolvedData>,
}

impl<'env> Resolved<'env> {
    pub fn new(ty: UnaliasedType<'env>, data: ResolvedData) -> Self {
        Self {
            ty,
            data: Some(data),
        }
    }

    pub fn from_type(ty: UnaliasedType<'env>) -> Self {
        Self { ty, data: None }
    }

    pub fn ty(&self) -> UnaliasedType<'env> {
        self.ty
    }
}

#[derive(Clone, Debug)]
pub enum UnaryImplicitCast {
    SpecializeBoolean(bool),
    SpecializeInteger(BigInt),
    SpecializeFloat(Option<NotNan<f64>>),
}

#[derive(Clone, Debug, From)]
pub enum ResolvedData {
    UnaryImplicitCast(UnaryImplicitCast),
    BinaryImplicitCast(BasicBinaryOperator),
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
