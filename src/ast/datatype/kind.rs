use super::{
    fmt_c_integer, AnonymousEnum, AnonymousStruct, AnoymousUnion, CInteger, FixedArray, FloatSize,
    FunctionPointer, IntegerBits, IntegerSign, Type,
};
use crate::source_files::Source;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub enum TypeKind {
    Boolean,
    Integer {
        bits: IntegerBits,
        sign: IntegerSign,
    },
    CInteger {
        integer: CInteger,
        sign: Option<IntegerSign>,
    },
    Float(FloatSize),
    Pointer(Box<Type>),
    FixedArray(Box<FixedArray>),
    PlainOldData(Box<Type>),
    Void,
    Named(String),
    AnonymousStruct(AnonymousStruct),
    AnonymousUnion(AnoymousUnion),
    AnonymousEnum(AnonymousEnum),
    FunctionPointer(FunctionPointer),
}

impl TypeKind {
    pub fn at(self, source: Source) -> Type {
        Type { kind: self, source }
    }

    pub fn allow_undeclared(&self) -> bool {
        // TODO: CLEANUP: This is a bad way of doing it, should `Named` have property for this?
        // This is very rarely needed though, so it's yet to be seen if that would be an improvement.
        if let TypeKind::Named(name) = self {
            if name.starts_with("struct<") {
                return true;
            }
        }
        false
    }
}

impl Display for &TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeKind::Boolean => {
                write!(f, "bool")?;
            }
            TypeKind::Integer { bits, sign } => {
                f.write_str(match (bits, sign) {
                    (IntegerBits::Normal, IntegerSign::Signed) => "int",
                    (IntegerBits::Normal, IntegerSign::Unsigned) => "uint",
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
            TypeKind::CInteger { integer, sign } => {
                fmt_c_integer(f, *integer, *sign)?;
            }
            TypeKind::Pointer(inner) => {
                write!(f, "ptr<{inner}>")?;
            }
            TypeKind::PlainOldData(inner) => {
                write!(f, "pod<{inner}>")?;
            }
            TypeKind::Void => {
                write!(f, "void")?;
            }
            TypeKind::Named(name) => {
                write!(f, "{name}")?;
            }
            TypeKind::Float(size) => f.write_str(match size {
                FloatSize::Normal => "float",
                FloatSize::Bits32 => "f32",
                FloatSize::Bits64 => "f64",
            })?,
            TypeKind::AnonymousStruct(..) => f.write_str("(anonymous struct)")?,
            TypeKind::AnonymousUnion(..) => f.write_str("(anonymous union)")?,
            TypeKind::AnonymousEnum(..) => f.write_str("(anonymous enum)")?,
            TypeKind::FixedArray(fixed_array) => {
                write!(f, "array<(amount), {}>", fixed_array.ast_type)?;
            }
            TypeKind::FunctionPointer(_function) => {
                write!(f, "(function pointer type)")?;
            }
        }

        Ok(())
    }
}
