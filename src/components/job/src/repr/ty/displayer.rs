use crate::{
    module_graph::ModuleView,
    repr::{Type, TypeArg, TypeHeadRestKind, TypeKind},
};
use diagnostics::minimal_filename;
use primitives::{FloatSize, IntegerBits, IntegerSign, fmt_c_integer};
use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fmt::Display,
};

#[derive(Clone)]
pub struct TypeDisplayerDisambiguation<'env> {
    ambiguous_names: HashSet<&'env str>,
}

impl<'env> TypeDisplayerDisambiguation<'env> {
    pub fn single<'a>(ty: &'a Type<'env>) -> Self {
        Self::new(std::iter::once(ty))
    }

    pub fn new<'a>(types: impl Iterator<Item = &'a Type<'env>>) -> Self
    where
        'env: 'a,
    {
        let mut states = HashMap::<&'env str, Result<TypeHeadRestKind, ()>>::new();

        // NOTE: We will also need to handle polymorph type disambiguation eventually,
        // I think it has to happen at a higher level than this though
        for ty in types {
            ty.kind.traverse_bfs((), &mut |_, kind| match kind {
                TypeKind::UserDefined(udt) => {
                    if let Some(existing) = states.get(udt.name) {
                        if let Ok(existing) = existing {
                            if *existing != udt.rest.kind {
                                states.insert(udt.name, Err(()));
                            }
                        }
                    } else {
                        states.insert(udt.name, Ok(udt.rest.kind));
                    }
                }
                _ => (),
            });
        }

        Self {
            ambiguous_names: states
                .into_iter()
                .filter_map(|(name, state)| state.is_err().then_some(name))
                .collect(),
        }
    }
}

pub struct TypeDisplayer<'a, 'b, 'c, 'env: 'a + 'b + 'c> {
    ty: &'a Type<'env>,
    view: &'b ModuleView<'env>,
    disambiguation: Cow<'c, TypeDisplayerDisambiguation<'env>>,
}

impl<'a, 'b, 'c, 'env: 'a + 'b + 'c> TypeDisplayer<'a, 'b, 'c, 'env> {
    pub fn new(
        ty: &'a Type<'env>,
        view: &'b ModuleView<'env>,
        disambiguation: Cow<'c, TypeDisplayerDisambiguation<'env>>,
    ) -> Self {
        Self {
            ty,
            view,
            disambiguation,
        }
    }
}

impl<'a, 'b, 'c, 'env: 'a + 'b + 'c> Display for TypeDisplayer<'a, 'b, 'c, 'env> {
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
                write!(f, "ptr'{}", inner.display(self.view, &self.disambiguation))
            }
            TypeKind::Deref(inner) => {
                write!(
                    f,
                    "deref'{}",
                    inner.display(self.view, &self.disambiguation)
                )
            }
            TypeKind::Void => write!(f, "void"),
            TypeKind::Never => write!(f, "never"),
            TypeKind::FixedArray(inner, count) => {
                write!(
                    f,
                    "array<{}, {}>",
                    count,
                    inner.display(self.view, &self.disambiguation)
                )
            }
            TypeKind::UserDefined(udt) => {
                if self.disambiguation.ambiguous_names.contains(udt.name) {
                    let filename = minimal_filename(
                        udt.rest.kind.source(),
                        self.view.compiler().source_files,
                        Some(self.view.compiler().project_root),
                    );
                    write!(f, "[\"{}\"]::", filename)?;
                }

                write!(f, "{}", udt.name)?;

                if udt.args.len() > 0 {
                    write!(f, "<")?;

                    for (i, arg) in udt.args.iter().enumerate() {
                        match arg {
                            TypeArg::Type(arg) => {
                                write!(f, "{}", arg.display(self.view, &self.disambiguation))?
                            }
                            TypeArg::Integer(big_int) => write!(f, "{}", big_int)?,
                        }

                        if i + 1 < udt.args.len() {
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
