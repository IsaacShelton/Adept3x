mod anonymous_enum;
mod constraint;
mod fixed_array;
mod func_ptr;

use super::Type;
use crate::{
    asg::{human_name::HumanName, Asg, EnumRef, StructRef, TraitRef, TypeAliasRef},
    ast::{fmt_c_integer, CInteger, FloatSize, IntegerBits, IntegerSign},
    source_files::Source,
    target::Target,
};
pub use constraint::Constraint;
use core::hash::Hash;
use derive_more::{IsVariant, Unwrap};
pub use fixed_array::FixedArray;
pub use func_ptr::FuncPtr;
use num::{BigInt, Zero};
use ordered_float::NotNan;
use std::fmt::Display;

#[derive(Clone, Debug, Hash, PartialEq, IsVariant, Unwrap)]
pub enum TypeKind {
    Unresolved,
    Boolean,
    Integer(IntegerBits, IntegerSign),
    CInteger(CInteger, Option<IntegerSign>),
    IntegerLiteral(BigInt),
    FloatLiteral(Option<NotNan<f64>>),
    Floating(FloatSize),
    Ptr(Box<Type>),
    Void,
    AnonymousStruct(),
    AnonymousUnion(),
    AnonymousEnum(),
    FixedArray(Box<FixedArray>),
    FuncPtr(FuncPtr),
    Enum(HumanName, EnumRef),
    Structure(HumanName, StructRef, Vec<Type>),
    TypeAlias(HumanName, TypeAliasRef),
    Polymorph(String, Vec<Constraint>),
    Trait(HumanName, TraitRef, Vec<Type>),
}

impl TypeKind {
    pub fn at(self, source: Source) -> Type {
        Type { kind: self, source }
    }

    pub fn contains_polymorph(&self) -> bool {
        match self {
            TypeKind::Unresolved => {
                panic!("asg::TypeKind::contains_polymorph was called on unresolved type")
            }
            TypeKind::Boolean
            | TypeKind::Integer(_, _)
            | TypeKind::CInteger(_, _)
            | TypeKind::IntegerLiteral(_)
            | TypeKind::FloatLiteral(_)
            | TypeKind::Floating(_) => false,
            TypeKind::Ptr(inner) => inner.kind.contains_polymorph(),
            TypeKind::Void => false,
            TypeKind::AnonymousStruct() => false,
            TypeKind::AnonymousUnion() => false,
            TypeKind::AnonymousEnum() => false,
            TypeKind::FixedArray(fixed_array) => fixed_array.inner.kind.contains_polymorph(),
            TypeKind::FuncPtr(_) => todo!(),
            TypeKind::Enum(_, _) => false,
            TypeKind::Structure(_, _, parameters) | TypeKind::Trait(_, _, parameters) => parameters
                .iter()
                .any(|parameter| parameter.kind.contains_polymorph()),
            TypeKind::TypeAlias(_, _) => false,
            TypeKind::Polymorph(_, _) => true,
        }
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
            | TypeKind::Ptr(_)
            | TypeKind::Structure(_, _, _)
            | TypeKind::Void
            | TypeKind::AnonymousStruct(..)
            | TypeKind::AnonymousUnion(..)
            | TypeKind::FixedArray(..)
            | TypeKind::FuncPtr(..)
            | TypeKind::Enum(_, _)
            | TypeKind::AnonymousEnum()
            | TypeKind::Polymorph(_, _)
            | TypeKind::Trait(_, _, _) => None,
        }
    }

    pub fn num_target_parameters(&self, asg: &Asg) -> usize {
        match self {
            TypeKind::Structure(_, struct_ref, _) => {
                asg.structs.get(*struct_ref).unwrap().params.len()
            }
            TypeKind::Trait(_, trait_ref, _) => asg.traits.get(*trait_ref).unwrap().params.len(),
            _ => 0,
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
            TypeKind::FloatLiteral(value) => {
                if let Some(value) = value {
                    write!(f, "float {}", value)?
                } else {
                    write!(f, "float NaN")?;
                }
            }
            TypeKind::Ptr(inner) => {
                write!(f, "ptr<{}>", **inner)?;
            }
            TypeKind::Void => f.write_str("void")?,
            TypeKind::Structure(name, _, parameters) => {
                write!(f, "{}", name)?;
                write_parameters(f, parameters)?;
            }
            TypeKind::AnonymousStruct() => f.write_str("anonymous-struct")?,
            TypeKind::AnonymousUnion() => f.write_str("anonymous-union")?,
            TypeKind::AnonymousEnum(..) => f.write_str("anonymous-enum")?,
            TypeKind::FixedArray(fixed_array) => {
                write!(f, "array<{}, {}>", fixed_array.size, fixed_array.inner.kind)?;
            }
            TypeKind::FuncPtr(..) => f.write_str("function-pointer-type")?,
            TypeKind::Enum(name, _) => write!(f, "{}", name)?,
            TypeKind::Polymorph(name, constaints) => {
                write!(f, "${}", name)?;

                if !constaints.is_empty() {
                    write!(f, ": ")?;
                }

                for constaint in constaints {
                    write!(f, "{}", constaint)?;
                }
            }
            TypeKind::Trait(name, _, parameters) => {
                write!(f, "{}", name)?;
                write_parameters(f, parameters)?;
            }
        }

        Ok(())
    }
}

fn write_parameters(f: &mut std::fmt::Formatter<'_>, parameters: &[Type]) -> std::fmt::Result {
    if !parameters.is_empty() {
        write!(f, "<")?;

        for (i, parameter) in parameters.iter().enumerate() {
            write!(f, "{}", parameter)?;

            if i + 1 < parameters.len() {
                write!(f, ", ")?;
            }
        }

        write!(f, ">")?;
    }

    Ok(())
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
