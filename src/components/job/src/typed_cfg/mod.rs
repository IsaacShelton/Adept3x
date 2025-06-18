/*
    ==================  components/job/src/typed_cfg/mod.rs  ==================
    Contains definitions for typing and resolving references for a CFG
    ---------------------------------------------------------------------------
*/

mod value;

use crate::repr::{Type, TypeKind};
use primitives::{FloatOrInteger, FloatOrSignLax, NumericMode, SignOrIndeterminate};
use source_files::Source;
pub use value::*;

#[derive(Clone, Debug)]
pub struct Typed<'env> {
    ty: Type<'env>,
    aux: Option<TypedAux>,
}

impl<'env> Typed<'env> {
    pub fn new(ty: Type<'env>, aux: TypedAux) -> Self {
        Self { ty, aux: Some(aux) }
    }
    pub fn from_type(ty: Type<'env>) -> Self {
        Self { ty, aux: None }
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
pub enum TypedAux {
    BasicBinaryOperator(BasicBinaryOperator),
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
