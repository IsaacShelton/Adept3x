mod displayer;

use crate::repr::TypeHeadRest;
use ast::IntegerKnown;
use derivative::Derivative;
use derive_more::IsVariant;
pub use displayer::*;
use num_bigint::BigInt;
use ordered_float::NotNan;
use primitives::{CInteger, FloatSize, IntegerBits, IntegerRigidity, IntegerSign, NumericMode};
use source_files::Source;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnaliasedType<'env>(pub &'env Type<'env>);

impl<'env> UnaliasedType<'env> {
    pub fn display(&self) -> TypeDisplayer<'_, 'env> {
        self.0.display()
    }
}

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

    pub fn display(&self) -> TypeDisplayer<'_, 'env> {
        TypeDisplayer::new(self)
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, IsVariant)]
pub enum TypeKind<'env> {
    // Mutable
    Deref(&'env Type<'env>),
    // Literals
    IntegerLiteral(&'env BigInt),
    IntegerLiteralInRange(&'env BigInt, &'env BigInt),
    FloatLiteral(Option<NotNan<f64>>),
    BooleanLiteral(bool),
    NullLiteral,
    AsciiCharLiteral(u8),
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
            | TypeKind::Floating(_)
            | TypeKind::AsciiCharLiteral(_) => false,
            TypeKind::Ptr(inner) | TypeKind::Deref(inner) => inner.kind.contains_polymorph(),
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
            | TypeKind::Floating(_)
            | TypeKind::AsciiCharLiteral(_) => false,
            TypeKind::Ptr(inner) | TypeKind::Deref(inner) => inner.kind.contains_type_alias(),
            TypeKind::Void | TypeKind::Never => false,
            TypeKind::FixedArray(inner, _) => inner.kind.contains_polymorph(),
            TypeKind::Polymorph(_) => true,
            TypeKind::UserDefined(user_defined_type) => user_defined_type.contains_type_alias(),
            TypeKind::DirectLabel(_) => false,
        }
    }

    pub fn bit_integer_sign(&self) -> Option<IntegerSign> {
        match self {
            TypeKind::BitInteger(_, integer_sign) => Some(*integer_sign),
            _ => None,
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UserDefinedType<'env> {
    pub name: &'env str,
    pub rest: TypeHeadRest<'env>,
    pub args: &'env [TypeArg<'env>],
}

impl<'env> UserDefinedType<'env> {
    pub fn contains_polymorph(&self) -> bool {
        self.args.iter().any(|arg| arg.contains_polymorph())
    }

    pub fn contains_type_alias(&self) -> bool {
        self.rest.kind.is_alias() || self.args.iter().any(|arg| arg.contains_type_alias())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnaliasedUserDefinedType<'env>(pub &'env UserDefinedType<'env>);
