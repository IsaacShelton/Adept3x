/*
    ==================  components/job/src/typed_cfg/mod.rs  ==================
    Contains definitions for typing and resolving references for a CFG
    ---------------------------------------------------------------------------
*/

mod value;

use crate::repr::{Type, TypeKind};
use num_bigint::BigInt;
use ordered_float::NotNan;
use primitives::{FloatOrInteger, FloatOrSignLax, NumericMode, SignOrIndeterminate};
use source_files::Source;
pub use value::*;

#[derive(Clone, Debug)]
pub struct Resolved<'env> {
    ty: Type<'env>,
    data: Option<ResolvedData>,
}

impl<'env> Resolved<'env> {
    pub fn new(ty: Type<'env>, data: ResolvedData) -> Self {
        Self {
            ty,
            data: Some(data),
        }
    }
    pub fn from_type(ty: Type<'env>) -> Self {
        Self { ty, data: None }
    }

    pub fn void(source: Source) -> Self {
        Self::from_type(Type {
            kind: TypeKind::Void,
            source,
        })
    }

    pub fn ty(&self) -> &Type<'env> {
        &self.ty
    }
}

#[derive(Clone, Debug)]
pub enum ResolvedData {
    BasicBinaryOperator(BasicBinaryOperator),
    SpecializeBoolean(bool),
    SpecializeInteger(BigInt),
    SpecializeFloat(Option<NotNan<f64>>),
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
