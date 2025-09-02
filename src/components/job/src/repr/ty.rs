use ast::IntegerKnown;
use ast_workspace::TypeDeclRef;
use derivative::Derivative;
use derive_more::{Display, IsVariant};
use num_bigint::BigInt;
use ordered_float::NotNan;
use primitives::{
    CInteger, FloatSize, IntegerBits, IntegerRigidity, IntegerSign, NumericMode, fmt_c_integer,
};
use source_files::Source;
use std::fmt::Display;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Display)]
pub struct UnaliasedType<'env>(pub &'env Type<'env>);

#[derive(Clone, Debug, Derivative)]
#[derivative(PartialEq, Eq, Hash)]
pub struct Type<'env> {
    pub kind: TypeKind<'env>,

    #[derivative(Hash = "ignore")]
    #[derivative(PartialEq = "ignore")]
    pub source: Source,
}

impl<'env> Type<'env> {
    pub fn numeric_mode(&self) -> Option<NumericMode> {
        self.kind.numeric_mode()
    }

    pub fn contains_polymorph(&self) -> bool {
        self.kind.contains_polymorph()
    }

    pub fn contains_type_alias(&self) -> bool {
        self.kind.contains_type_alias()
    }
}

impl<'env> Display for Type<'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.kind.fmt(f)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeArg<'env> {
    Type(&'env Type<'env>),
    Integer(BigInt),
}

impl<'env> TypeArg<'env> {
    pub fn contains_polymorph(&self) -> bool {
        match self {
            TypeArg::Type(ty) => ty.contains_polymorph(),
            TypeArg::Integer(_) => false,
        }
    }

    pub fn contains_type_alias(&self) -> bool {
        match self {
            TypeArg::Type(ty) => ty.contains_type_alias(),
            TypeArg::Integer(_) => false,
        }
    }
}

impl<'env> Display for TypeArg<'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeArg::Type(ty) => write!(f, "{}", ty),
            TypeArg::Integer(integer) => write!(f, "{}", integer),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, IsVariant)]
pub enum TypeKind<'env> {
    // Literals
    IntegerLiteral(&'env BigInt),
    IntegerLiteralInRange(&'env BigInt, &'env BigInt),
    FloatLiteral(Option<NotNan<f64>>),
    BooleanLiteral(bool),
    NullLiteral,
    // Boolean
    Boolean,
    // Integer
    BitInteger(IntegerBits, IntegerSign),
    CInteger(CInteger, Option<IntegerSign>),
    SizeInteger(IntegerSign),
    // Floats
    Floating(FloatSize),
    // Pointers
    Ptr(&'env Type<'env>),
    // Void
    Void,
    // Never
    Never,
    // Fixed-Size Array
    FixedArray(&'env Type<'env>, usize),
    // User-Defined
    UserDefined(UserDefinedType<'env>),
    // Polymorph
    Polymorph(&'env str),
    // Goto Label
    DirectLabel(&'env str),
    // NOTE: Once we want to support computed GOTOs, we can add the following:
    // IndirectLabel(&'env [&'env str]),
}

impl<'env> TypeKind<'env> {
    pub fn at(self, source: Source) -> Type<'env> {
        Type { kind: self, source }
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            Self::IntegerLiteral(..)
                | Self::BitInteger(..)
                | Self::CInteger(..)
                | Self::SizeInteger(..)
        )
    }

    pub fn numeric_mode(&self) -> Option<NumericMode> {
        match self {
            Self::BitInteger(_, sign) => Some(NumericMode::Integer(*sign)),
            Self::CInteger(c_integer, sign) => Some(if let Some(sign) = sign {
                NumericMode::Integer(*sign)
            } else {
                NumericMode::LooseIndeterminateSignInteger(*c_integer)
            }),
            Self::Floating(_) => Some(NumericMode::Float),
            _ => None,
        }
    }

    pub fn contains_polymorph(&self) -> bool {
        match self {
            TypeKind::Boolean
            | TypeKind::NullLiteral
            | TypeKind::BooleanLiteral(_)
            | TypeKind::BitInteger(_, _)
            | TypeKind::CInteger(_, _)
            | TypeKind::SizeInteger(_)
            | TypeKind::IntegerLiteral(_)
            | TypeKind::IntegerLiteralInRange(..)
            | TypeKind::FloatLiteral(_)
            | TypeKind::Floating(_) => false,
            TypeKind::Ptr(inner) => inner.kind.contains_polymorph(),
            TypeKind::Void | TypeKind::Never => false,
            TypeKind::FixedArray(inner, _) => inner.kind.contains_polymorph(),
            TypeKind::Polymorph(_) => true,
            TypeKind::UserDefined(user_defined_type) => user_defined_type.contains_polymorph(),
            TypeKind::DirectLabel(_) => false,
        }
    }

    pub fn contains_type_alias(&self) -> bool {
        match self {
            TypeKind::Boolean
            | TypeKind::NullLiteral
            | TypeKind::BooleanLiteral(_)
            | TypeKind::BitInteger(_, _)
            | TypeKind::CInteger(_, _)
            | TypeKind::SizeInteger(_)
            | TypeKind::IntegerLiteral(_)
            | TypeKind::IntegerLiteralInRange(..)
            | TypeKind::FloatLiteral(_)
            | TypeKind::Floating(_) => false,
            TypeKind::Ptr(inner) => inner.kind.contains_type_alias(),
            TypeKind::Void | TypeKind::Never => false,
            TypeKind::FixedArray(inner, _) => inner.kind.contains_polymorph(),
            TypeKind::Polymorph(_) => true,
            TypeKind::UserDefined(user_defined_type) => user_defined_type.contains_type_alias(),
            TypeKind::DirectLabel(_) => false,
        }
    }
}

impl<'env> From<&IntegerKnown> for TypeKind<'env> {
    fn from(value: &IntegerKnown) -> Self {
        match value.rigidity {
            IntegerRigidity::Fixed(bits, sign) => TypeKind::BitInteger(bits, sign),
            IntegerRigidity::Loose(c_integer, sign) => TypeKind::CInteger(c_integer, sign),
            IntegerRigidity::Size(sign) => TypeKind::SizeInteger(sign),
        }
    }
}

impl<'env> Display for TypeKind<'env> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeKind::IntegerLiteral(integer) => write!(f, "integer {}", integer),
            TypeKind::IntegerLiteralInRange(min, max) => {
                write!(f, "integer {}..={}", min, max)
            }
            TypeKind::FloatLiteral(Some(float)) => write!(f, "float {}", float),
            TypeKind::FloatLiteral(None) => write!(f, "float NaN"),
            TypeKind::Boolean => write!(f, "bool"),
            TypeKind::NullLiteral => write!(f, "null"),
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
                write!(f, "ptr<{}>", inner)
            }
            TypeKind::Void => write!(f, "void"),
            TypeKind::Never => write!(f, "never"),
            TypeKind::FixedArray(inner, count) => write!(f, "array<{}, {}>", count, inner),
            TypeKind::UserDefined(user_defined_type) => {
                write!(f, "{}", user_defined_type.name)?;

                if user_defined_type.args.len() > 0 {
                    write!(f, "<")?;

                    for (i, arg) in user_defined_type.args.iter().enumerate() {
                        write!(f, "{}", arg)?;

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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UserDefinedType<'env> {
    pub name: &'env str,
    pub type_decl_ref: TypeDeclRef,
    pub args: &'env [TypeArg<'env>],
}

impl<'env> UserDefinedType<'env> {
    pub fn contains_polymorph(&self) -> bool {
        self.args.iter().any(|arg| arg.contains_polymorph())
    }

    pub fn contains_type_alias(&self) -> bool {
        self.type_decl_ref.is_alias() || self.args.iter().any(|arg| arg.contains_type_alias())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnaliasedUserDefinedType<'env>(pub &'env UserDefinedType<'env>);
