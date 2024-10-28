mod anonymous_enum;
mod fixed_array;
mod function_pointer;

use super::Type;
use crate::{
    ast::{fmt_c_integer, CInteger, FloatSize, IntegerBits, IntegerSign},
    resolved::{human_name::HumanName, EnumRef, StructureRef, TypeAliasRef},
    source_files::Source,
    target::Target,
};
pub use anonymous_enum::AnonymousEnum;
use derive_more::{IsVariant, Unwrap};
pub use fixed_array::FixedArray;
pub use function_pointer::FunctionPointer;
use num::{BigInt, Zero};
use std::fmt::Display;

#[derive(Clone, Debug, PartialEq, IsVariant, Unwrap)]
pub enum TypeKind {
    Unresolved,
    Boolean,
    Integer(IntegerBits, IntegerSign),
    CInteger(CInteger, Option<IntegerSign>),
    IntegerLiteral(BigInt),
    FloatLiteral(f64),
    Floating(FloatSize),
    Pointer(Box<Type>),
    Void,
    AnonymousStruct(),
    AnonymousUnion(),
    AnonymousEnum(AnonymousEnum),
    FixedArray(Box<FixedArray>),
    FunctionPointer(FunctionPointer),
    Enum(HumanName, EnumRef),
    Structure(HumanName, StructureRef),
    TypeAlias(HumanName, TypeAliasRef),
    Polymorph(String),
}

impl TypeKind {
    pub fn at(self, source: Source) -> Type {
        Type { kind: self, source }
    }

    pub fn sign(&self, target: Option<&Target>) -> Option<IntegerSign> {
        match self {
            TypeKind::Boolean => Some(IntegerSign::Unsigned),
            TypeKind::Integer(_, sign) => Some(*sign),
            TypeKind::IntegerLiteral(value) => Some(if value >= &BigInt::zero() {
                IntegerSign::Unsigned
            } else {
                IntegerSign::Signed
            }),
            TypeKind::CInteger(integer, sign) => {
                sign.or_else(|| target.map(|target| target.default_c_integer_sign(*integer)))
            }
            TypeKind::TypeAlias(_, _type_ref) => todo!(),
            TypeKind::Unresolved => panic!(),
            TypeKind::Floating(_)
            | TypeKind::FloatLiteral(_)
            | TypeKind::Pointer(_)
            | TypeKind::Structure(_, _)
            | TypeKind::Void
            | TypeKind::AnonymousStruct(..)
            | TypeKind::AnonymousUnion(..)
            | TypeKind::FixedArray(..)
            | TypeKind::FunctionPointer(..)
            | TypeKind::Enum(_, _)
            | TypeKind::AnonymousEnum(_)
            | TypeKind::Polymorph(_) => None,
        }
    }
}

impl Display for TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeKind::Unresolved => panic!("cannot display unresolved type"),
            TypeKind::TypeAlias(name, _) => write!(f, "{}", name)?,
            TypeKind::Boolean => write!(f, "bool")?,
            TypeKind::Integer(bits, sign) => {
                f.write_str(match (bits, sign) {
                    (IntegerBits::Bits8, IntegerSign::Signed) => "i8",
                    (IntegerBits::Bits8, IntegerSign::Unsigned) => "u8",
                    (IntegerBits::Bits16, IntegerSign::Signed) => "i16",
                    (IntegerBits::Bits16, IntegerSign::Unsigned) => "u16",
                    (IntegerBits::Bits32, IntegerSign::Signed) => "i32",
                    (IntegerBits::Bits32, IntegerSign::Unsigned) => "u32",
                    (IntegerBits::Bits64, IntegerSign::Signed) => "i64",
                    (IntegerBits::Bits64, IntegerSign::Unsigned) => "u64",
                })?;
            }
            TypeKind::CInteger(integer, sign) => {
                fmt_c_integer(f, *integer, *sign)?;
            }
            TypeKind::IntegerLiteral(value) => {
                write!(f, "integer {}", value)?;
            }
            TypeKind::Floating(size) => match size {
                FloatSize::Bits32 => f.write_str("f32")?,
                FloatSize::Bits64 => f.write_str("f64")?,
            },
            TypeKind::FloatLiteral(value) => write!(f, "float {}", value)?,
            TypeKind::Pointer(inner) => {
                write!(f, "ptr<{}>", **inner)?;
            }
            TypeKind::Void => f.write_str("void")?,
            TypeKind::Structure(name, _) => write!(f, "{}", name)?,
            TypeKind::AnonymousStruct() => f.write_str("anonymous-struct")?,
            TypeKind::AnonymousUnion() => f.write_str("anonymous-union")?,
            TypeKind::AnonymousEnum(..) => f.write_str("anonymous-enum")?,
            TypeKind::FixedArray(fixed_array) => {
                write!(f, "array<{}, {}>", fixed_array.size, fixed_array.inner.kind)?;
            }
            TypeKind::FunctionPointer(..) => f.write_str("function-pointer-type")?,
            TypeKind::Enum(name, _) => write!(f, "{}", name)?,
            TypeKind::Polymorph(name) => write!(f, "${}", name)?,
        }

        Ok(())
    }
}

impl TypeKind {
    pub fn i8() -> Self {
        Self::Integer(IntegerBits::Bits8, IntegerSign::Signed)
    }

    pub fn u8() -> Self {
        Self::Integer(IntegerBits::Bits8, IntegerSign::Unsigned)
    }

    pub fn i16() -> Self {
        Self::Integer(IntegerBits::Bits16, IntegerSign::Signed)
    }

    pub fn u16() -> Self {
        Self::Integer(IntegerBits::Bits16, IntegerSign::Unsigned)
    }

    pub fn i32() -> Self {
        Self::Integer(IntegerBits::Bits32, IntegerSign::Signed)
    }

    pub fn u32() -> Self {
        Self::Integer(IntegerBits::Bits32, IntegerSign::Unsigned)
    }

    pub fn i64() -> Self {
        Self::Integer(IntegerBits::Bits64, IntegerSign::Signed)
    }

    pub fn u64() -> Self {
        Self::Integer(IntegerBits::Bits64, IntegerSign::Unsigned)
    }

    pub fn f32() -> Self {
        Self::Floating(FloatSize::Bits32)
    }

    pub fn f64() -> Self {
        Self::Floating(FloatSize::Bits64)
    }

    pub fn signed(bits: IntegerBits) -> Self {
        Self::Integer(bits, IntegerSign::Signed)
    }
    pub fn unsigned(bits: IntegerBits) -> Self {
        Self::Integer(bits, IntegerSign::Unsigned)
    }

    pub fn is_integer_like(&self) -> bool {
        matches!(
            self,
            Self::Integer(..) | Self::IntegerLiteral(..) | Self::CInteger(..)
        )
    }

    pub fn is_float_like(&self) -> bool {
        matches!(self, Self::Floating(..) | Self::FloatLiteral(..))
    }

    pub fn is_ambiguous(&self) -> bool {
        self.is_integer_literal()
    }
}
