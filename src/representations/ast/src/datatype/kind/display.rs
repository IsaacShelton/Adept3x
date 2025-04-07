use super::TypeKind;
use crate::TypeArg;
use primitives::{FloatSize, IntegerBits, IntegerSign, fmt_c_integer};
use std::fmt::Display;

impl Display for &TypeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeKind::Boolean => {
                write!(f, "bool")?;
            }
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
            TypeKind::Ptr(inner) => {
                write!(f, "ptr<{inner}>")?;
            }
            TypeKind::Void => {
                write!(f, "void")?;
            }
            TypeKind::Never => {
                write!(f, "never")?;
            }
            TypeKind::Named(name, arguments) => {
                write!(f, "{name}")?;

                if !arguments.is_empty() {
                    write!(f, "<")?;

                    for (i, argument) in arguments.iter().enumerate() {
                        match argument {
                            TypeArg::Type(ty) => write!(f, "{}", ty)?,
                            TypeArg::Expr(expr) => write!(f, "({:?})", expr)?, // TODO: Implement display for ast::Expr
                        }

                        if i + 1 < arguments.len() {
                            write!(f, ", ")?;
                        }
                    }

                    write!(f, ">")?;
                }
            }
            TypeKind::Floating(size) => f.write_str(match size {
                FloatSize::Bits32 => "f32",
                FloatSize::Bits64 => "f64",
            })?,
            TypeKind::AnonymousStruct(..) => f.write_str("(anonymous struct)")?,
            TypeKind::AnonymousUnion(..) => f.write_str("(anonymous union)")?,
            TypeKind::AnonymousEnum(..) => f.write_str("(anonymous enum)")?,
            TypeKind::FixedArray(fixed_array) => {
                write!(f, "array<(amount), {}>", fixed_array.ast_type)?;
            }
            TypeKind::FuncPtr(_function) => {
                write!(f, "(function pointer type)")?;
            }
            TypeKind::Polymorph(polymorph) => {
                write!(f, "${}", polymorph)?;
            }
        }

        Ok(())
    }
}
