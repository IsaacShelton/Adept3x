mod anonymous_enum;
mod fixed_array;
mod func_ptr;

use crate::{Asg, EnumRef, StructRef, TraitRef, Type, TypeAliasRef, human_name::HumanName};
pub use anonymous_enum::AnonymousEnum;
use ast::IntegerKnown;
use core::hash::Hash;
use derive_more::{IsVariant, Unwrap};
pub use fixed_array::FixedArray;
pub use func_ptr::FuncPtr;
use num::{BigInt, Zero};
use ordered_float::NotNan;
use primitives::{
    CInteger, FloatSize, IntegerBits, IntegerRigidity, IntegerSign, NumericMode, fmt_c_integer,
};
use source_files::Source;
use std::{borrow::Cow, fmt::Display};
use target::Target;

#[derive(Clone, Debug, Hash, PartialEq, IsVariant, Unwrap)]
pub enum TypeKind {
    Unresolved,
    Boolean,
    Integer(IntegerBits, IntegerSign),
    CInteger(CInteger, Option<IntegerSign>),
    SizeInteger(IntegerSign),
    IntegerLiteral(BigInt),
    FloatLiteral(Option<NotNan<f64>>),
    Floating(FloatSize),
    Ptr(Box<Type>),
    Void,
    Never,
    AnonymousStruct(),
    AnonymousUnion(),
    AnonymousEnum(Box<AnonymousEnum>),
    FixedArray(Box<FixedArray>),
    FuncPtr(FuncPtr),
    Enum(HumanName, EnumRef),
    Structure(HumanName, StructRef, Vec<Type>),
    TypeAlias(HumanName, TypeAliasRef, Vec<Type>),
    Polymorph(String),
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
            | TypeKind::SizeInteger(_)
            | TypeKind::IntegerLiteral(_)
            | TypeKind::FloatLiteral(_)
            | TypeKind::Floating(_) => false,
            TypeKind::Ptr(inner) => inner.kind.contains_polymorph(),
            TypeKind::Void | TypeKind::Never => false,
            TypeKind::AnonymousStruct() => false,
            TypeKind::AnonymousUnion() => false,
            TypeKind::AnonymousEnum(_) => false,
            TypeKind::FixedArray(fixed_array) => fixed_array.inner.kind.contains_polymorph(),
            TypeKind::FuncPtr(_) => todo!("contains_polymorph FuncPtr"),
            TypeKind::Enum(_, _) => false,
            TypeKind::Structure(_, _, parameters)
            | TypeKind::Trait(_, _, parameters)
            | TypeKind::TypeAlias(_, _, parameters) => parameters
                .iter()
                .any(|parameter| parameter.kind.contains_polymorph()),
            TypeKind::Polymorph(_) => true,
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
            TypeKind::SizeInteger(sign) => Some(*sign),
            TypeKind::TypeAlias(_, _, _) => panic!("sign of type alias"),
            TypeKind::Unresolved => panic!(),
            TypeKind::AnonymousEnum(enumeration) => enumeration.backing_type.kind.sign(target),
            TypeKind::Floating(_)
            | TypeKind::FloatLiteral(_)
            | TypeKind::Ptr(_)
            | TypeKind::Structure(_, _, _)
            | TypeKind::Void
            | TypeKind::Never
            | TypeKind::AnonymousStruct(..)
            | TypeKind::AnonymousUnion(..)
            | TypeKind::FixedArray(..)
            | TypeKind::FuncPtr(..)
            | TypeKind::Enum(_, _)
            | TypeKind::Polymorph(_)
            | TypeKind::Trait(_, _, _) => None,
        }
    }

    pub fn num_target_parameters(&self, asg: &Asg) -> usize {
        match self {
            TypeKind::Structure(_, struct_ref, _) => {
                asg.structs.get(*struct_ref).unwrap().params.len()
            }
            TypeKind::Trait(_, trait_ref, _) => asg.traits.get(*trait_ref).unwrap().params.len(),
            TypeKind::TypeAlias(_, type_alias_ref, _) => {
                asg.type_aliases.get(*type_alias_ref).unwrap().params.len()
            }
            _ => 0,
        }
    }

    pub fn numeric_mode(unified_type: &Type) -> Option<NumericMode> {
        match &unified_type.kind {
            TypeKind::Integer(_, sign) => Some(NumericMode::Integer(*sign)),
            TypeKind::CInteger(c_integer, sign) => {
                if let Some(sign) = sign {
                    Some(NumericMode::Integer(*sign))
                } else {
                    Some(NumericMode::LooseIndeterminateSignInteger(*c_integer))
                }
            }
            TypeKind::Floating(_) => Some(NumericMode::Float),
            _ => None,
        }
    }

    pub fn for_each_polymorph(&self, f: &mut impl FnMut(&str) -> ()) {
        match self {
            TypeKind::Unresolved => panic!("unresolved type"),
            TypeKind::Boolean
            | TypeKind::Integer(_, _)
            | TypeKind::CInteger(_, _)
            | TypeKind::SizeInteger(_)
            | TypeKind::IntegerLiteral(_)
            | TypeKind::FloatLiteral(_)
            | TypeKind::Floating(_) => (),
            TypeKind::Ptr(inner) => inner.kind.for_each_polymorph(f),
            TypeKind::Void
            | TypeKind::Never
            | TypeKind::AnonymousStruct()
            | TypeKind::AnonymousUnion()
            | TypeKind::AnonymousEnum(_) => (),
            TypeKind::FixedArray(fixed_array) => fixed_array.inner.kind.for_each_polymorph(f),
            TypeKind::FuncPtr(func) => {
                for param in func.params.required.iter() {
                    param.ty.kind.for_each_polymorph(f);
                }
                func.return_type.kind.for_each_polymorph(f);
            }
            TypeKind::Enum(_, _) => (),
            TypeKind::Structure(_, _, params) | TypeKind::TypeAlias(_, _, params) => {
                for param in params.iter() {
                    param.kind.for_each_polymorph(f);
                }
            }
            TypeKind::Polymorph(name) => f(name),
            TypeKind::Trait(_, _, params) => {
                for param in params.iter() {
                    param.kind.for_each_polymorph(f);
                }
            }
        }
    }

    pub fn map_type_params<E>(
        &self,
        mut mapper: impl FnMut(TypeParam) -> Result<TypeParam, E>,
    ) -> Result<Cow<Self>, TypeParamError<E>> {
        match self {
            TypeKind::Unresolved
            | TypeKind::Boolean
            | TypeKind::Integer(_, _)
            | TypeKind::CInteger(_, _)
            | TypeKind::SizeInteger(_)
            | TypeKind::IntegerLiteral(_)
            | TypeKind::FloatLiteral(_)
            | TypeKind::Floating(_) => Ok(Cow::Borrowed(self)),
            TypeKind::Ptr(inner) => {
                let TypeParam::Type(inner) = mapper(TypeParam::Type(Cow::Borrowed(inner)))
                    .map_err(TypeParamError::MappingError)?
                else {
                    return Err(TypeParamError::ExpectedType { index: 0 });
                };

                Ok(Cow::Owned(Self::Ptr(Box::new(inner.into_owned()))))
            }
            TypeKind::Void
            | TypeKind::Never
            | TypeKind::AnonymousStruct()
            | TypeKind::AnonymousUnion()
            | TypeKind::AnonymousEnum(_) => Ok(Cow::Borrowed(self)),
            TypeKind::FixedArray(fixed_array) => {
                let TypeParam::Size(size) = mapper(TypeParam::Size(fixed_array.size))
                    .map_err(TypeParamError::MappingError)?
                else {
                    return Err(TypeParamError::ExpectedSize { index: 0 });
                };

                if size != fixed_array.size {
                    return Err(TypeParamError::ExpectedSizeValue {
                        index: 0,
                        value: fixed_array.size,
                    });
                }

                let TypeParam::Type(inner) =
                    mapper(TypeParam::Type(Cow::Borrowed(&fixed_array.inner)))
                        .map_err(TypeParamError::MappingError)?
                else {
                    return Err(TypeParamError::ExpectedType { index: 1 });
                };

                Ok(Cow::Owned(Self::FixedArray(Box::new(FixedArray {
                    size,
                    inner: inner.into_owned(),
                }))))
            }
            TypeKind::FuncPtr(_) => todo!("map_type_params FuncPtr"),
            TypeKind::Enum(_, _) => Ok(Cow::Borrowed(self)),
            TypeKind::Structure(human_name, struct_ref, type_args) => {
                if type_args.is_empty() {
                    return Ok(Cow::Borrowed(self));
                }

                let mut mapped = vec![];

                for (i, type_arg) in type_args.iter().enumerate() {
                    let TypeParam::Type(inner) = mapper(TypeParam::Type(Cow::Borrowed(type_arg)))
                        .map_err(TypeParamError::MappingError)?
                    else {
                        return Err(TypeParamError::ExpectedType { index: i });
                    };

                    mapped.push(inner.into_owned());
                }

                Ok(Cow::Owned(Self::Structure(
                    human_name.clone(),
                    *struct_ref,
                    mapped,
                )))
            }
            TypeKind::TypeAlias(human_name, type_alias_ref, type_args) => {
                if type_args.is_empty() {
                    return Ok(Cow::Borrowed(self));
                }

                let mut mapped = vec![];

                for (i, type_arg) in type_args.iter().enumerate() {
                    let TypeParam::Type(inner) = mapper(TypeParam::Type(Cow::Borrowed(type_arg)))
                        .map_err(TypeParamError::MappingError)?
                    else {
                        return Err(TypeParamError::ExpectedType { index: i });
                    };

                    mapped.push(inner.into_owned());
                }

                Ok(Cow::Owned(Self::TypeAlias(
                    human_name.clone(),
                    *type_alias_ref,
                    mapped,
                )))
            }
            TypeKind::Polymorph(_) => Ok(Cow::Borrowed(self)),
            TypeKind::Trait(human_name, trait_ref, type_args) => {
                if type_args.is_empty() {
                    return Ok(Cow::Borrowed(self));
                }

                let mut mapped = vec![];

                for (i, type_arg) in type_args.iter().enumerate() {
                    let TypeParam::Type(inner) = mapper(TypeParam::Type(Cow::Borrowed(type_arg)))
                        .map_err(TypeParamError::MappingError)?
                    else {
                        return Err(TypeParamError::ExpectedType { index: i });
                    };

                    mapped.push(inner.into_owned());
                }

                Ok(Cow::Owned(Self::Trait(
                    human_name.clone(),
                    *trait_ref,
                    mapped,
                )))
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum TypeParam<'a> {
    Size(u64),
    Type(Cow<'a, Type>),
}

#[derive(Clone, Debug)]
pub enum TypeParamError<E> {
    MappingError(E),
    ExpectedType { index: usize },
    ExpectedSize { index: usize },
    ExpectedSizeValue { index: usize, value: u64 },
}

impl Display for TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeKind::Unresolved => panic!("cannot display unresolved type"),
            TypeKind::TypeAlias(name, _, type_args) => {
                write!(f, "{}", name)?;
                write_parameters(f, type_args)?;
            }
            TypeKind::Boolean => write!(f, "bool")?,
            TypeKind::Integer(bits, sign) => f.write_str(match (bits, sign) {
                (IntegerBits::Bits8, IntegerSign::Signed) => "i8",
                (IntegerBits::Bits8, IntegerSign::Unsigned) => "u8",
                (IntegerBits::Bits16, IntegerSign::Signed) => "i16",
                (IntegerBits::Bits16, IntegerSign::Unsigned) => "u16",
                (IntegerBits::Bits32, IntegerSign::Signed) => "i32",
                (IntegerBits::Bits32, IntegerSign::Unsigned) => "u32",
                (IntegerBits::Bits64, IntegerSign::Signed) => "i64",
                (IntegerBits::Bits64, IntegerSign::Unsigned) => "u64",
            })?,
            TypeKind::CInteger(integer, sign) => {
                fmt_c_integer(f, *integer, *sign)?;
            }
            TypeKind::SizeInteger(sign) => f.write_str(match sign {
                IntegerSign::Signed => "isize",
                IntegerSign::Unsigned => "usize",
            })?,
            TypeKind::IntegerLiteral(value) => {
                write!(f, "integer {}", value)?;
            }
            TypeKind::Floating(size) => f.write_str(match size {
                FloatSize::Bits32 => "f32",
                FloatSize::Bits64 => "f64",
            })?,
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
            TypeKind::Never => f.write_str("never")?,
            TypeKind::Structure(name, _, type_args) => {
                write!(f, "{}", name)?;
                write_parameters(f, type_args)?;
            }
            TypeKind::AnonymousStruct() => f.write_str("anonymous-struct")?,
            TypeKind::AnonymousUnion() => f.write_str("anonymous-union")?,
            TypeKind::AnonymousEnum(..) => f.write_str("anonymous-enum")?,
            TypeKind::FixedArray(fixed_array) => {
                write!(f, "array<{}, {}>", fixed_array.size, fixed_array.inner.kind)?;
            }
            TypeKind::FuncPtr(..) => f.write_str("function-pointer-type")?,
            TypeKind::Enum(name, _) => write!(f, "{}", name)?,
            TypeKind::Polymorph(name) => {
                write!(f, "${}", name)?;
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
            Self::Integer(..)
                | Self::IntegerLiteral(..)
                | Self::CInteger(..)
                | Self::SizeInteger(..)
        )
    }

    pub fn is_float_like(&self) -> bool {
        matches!(self, Self::Floating(..) | Self::FloatLiteral(..))
    }

    pub fn is_ambiguous(&self) -> bool {
        self.is_integer_literal()
    }
}

impl From<&IntegerKnown> for TypeKind {
    fn from(value: &IntegerKnown) -> Self {
        match value.rigidity {
            IntegerRigidity::Fixed(bits, sign) => TypeKind::Integer(bits, sign),
            IntegerRigidity::Loose(c_integer, sign) => TypeKind::CInteger(c_integer, sign),
            IntegerRigidity::Size(sign) => TypeKind::SizeInteger(sign),
        }
    }
}
