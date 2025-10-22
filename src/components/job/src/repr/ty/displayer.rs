use crate::{
    module_graph::ModuleView,
    repr::{Type, TypeArg, TypeKind},
};
use primitives::{FloatSize, IntegerBits, IntegerSign, fmt_c_integer};
use std::fmt::Display;

pub struct TypeDisplayer<'a, 'b, 'env: 'a + 'b> {
    ty: &'a Type<'env>,
    view: &'b ModuleView<'env>,
}

impl<'a, 'b, 'env: 'a + 'b> TypeDisplayer<'a, 'b, 'env> {
    pub fn new(ty: &'a Type<'env>, view: &'b ModuleView<'env>) -> Self {
        Self { ty, view }
    }
}

impl<'a, 'b, 'env: 'a + 'b> Display for TypeDisplayer<'a, 'b, 'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = &self.ty.kind;

        match kind {
            TypeKind::IntegerLiteral(integer) => write!(f, "integer {}", integer),
            TypeKind::IntegerLiteralInRange(min, max) => {
                write!(f, "integer {}..={}", min, max)
            }
            TypeKind::FloatLiteral(Some(float)) => write!(f, "float {}", float),
            TypeKind::FloatLiteral(None) => write!(f, "float NaN"),
            TypeKind::Boolean => write!(f, "bool"),
            TypeKind::NullLiteral => write!(f, "null"),
            TypeKind::AsciiCharLiteral(c) => {
                if c.is_ascii_graphic() || *c == b' ' {
                    write!(f, "'{}'", *c)
                } else if *c == b'\n' {
                    write!(f, "'\\n'")
                } else if *c == b'\t' {
                    write!(f, "'\\t'")
                } else if *c == b'\r' {
                    write!(f, "'\\r'")
                } else if *c == 0x1B {
                    write!(f, "'\\e'")
                } else if *c == b'\0' {
                    write!(f, "'\\0'")
                } else {
                    write!(f, "'\\x{:02X}'", *c as i32)
                }
            }
            TypeKind::BooleanLiteral(value) => write!(f, "bool {}", value),
            TypeKind::BitInteger(bits, sign) => f.write_str(match (bits, sign) {
                (IntegerBits::Bits8, IntegerSign::Signed) => "i8",
                (IntegerBits::Bits8, IntegerSign::Unsigned) => "u8",
                (IntegerBits::Bits16, IntegerSign::Signed) => "i16",
                (IntegerBits::Bits16, IntegerSign::Unsigned) => "u16",
                (IntegerBits::Bits32, IntegerSign::Signed) => "i32",
                (IntegerBits::Bits32, IntegerSign::Unsigned) => "u32",
                (IntegerBits::Bits64, IntegerSign::Signed) => "i64",
                (IntegerBits::Bits64, IntegerSign::Unsigned) => "u64",
            }),
            TypeKind::CInteger(cinteger, sign) => fmt_c_integer(f, *cinteger, *sign),
            TypeKind::SizeInteger(sign) => f.write_str(match sign {
                IntegerSign::Signed => "isize",
                IntegerSign::Unsigned => "usize",
            }),

            TypeKind::Floating(float_size) => f.write_str(match float_size {
                FloatSize::Bits32 => "f32",
                FloatSize::Bits64 => "f64",
            }),
            TypeKind::Ptr(inner) => {
                write!(f, "ptr'{}", inner.display(self.view))
            }
            TypeKind::Deref(inner) => {
                write!(f, "deref'{}", inner.display(self.view))
            }
            TypeKind::Void => write!(f, "void"),
            TypeKind::Never => write!(f, "never"),
            TypeKind::FixedArray(inner, count) => {
                write!(f, "array<{}, {}>", count, inner.display(self.view))
            }
            TypeKind::UserDefined(user_defined_type) => {
                write!(f, "{}", user_defined_type.name)?;

                if user_defined_type.args.len() > 0 {
                    write!(f, "<")?;

                    for (i, arg) in user_defined_type.args.iter().enumerate() {
                        match arg {
                            TypeArg::Type(arg) => write!(f, "{}", arg.display(self.view))?,
                            TypeArg::Integer(big_int) => write!(f, "{}", big_int)?,
                        }

                        if i + 1 < user_defined_type.args.len() {
                            write!(f, ", ")?;
                        }
                    }

                    write!(f, ">")?;
                }

                Ok(())
            }
            TypeKind::Polymorph(polymorph) => write!(f, "${}", polymorph),
            // NOTE: Direct labels are not "real types". They are zero-sized, compile-time-known,
            // and can't be named, returned, etc.
            TypeKind::DirectLabel(name) => write!(f, "@{}@", name),
        }
    }
}
