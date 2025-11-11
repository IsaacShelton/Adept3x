mod displayer;

use crate::{
    module_graph::ModuleView,
    repr::{TypeHeadRest, TypeHeadRestKind},
};
use ast::IntegerKnown;
use derivative::Derivative;
use derive_more::IsVariant;
pub use displayer::*;
use num_bigint::BigInt;
use ordered_float::NotNan;
use primitives::{CInteger, FloatSize, IntegerBits, IntegerRigidity, IntegerSign, NumericMode};
use source_files::Source;
use std::borrow::Cow;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnaliasedType<'env>(pub &'env Type<'env>);

impl<'env> UnaliasedType<'env> {
    pub fn display<'a, 'b, 'c>(
        &'a self,
        view: &'b ModuleView<'env>,
        disambiguation: &'c TypeDisplayerDisambiguation<'env>,
    ) -> TypeDisplayer<'a, 'b, 'c, 'env> {
        self.0.display(view, disambiguation)
    }

    pub fn display_one<'a, 'b, 'c>(
        &'a self,
        view: &'b ModuleView<'env>,
    ) -> TypeDisplayer<'a, 'b, 'c, 'env> {
        self.0.display_one(view)
    }

    pub fn without_leading_derefs(&self, count: usize) -> UnaliasedType<'env> {
        let mut ty = self.0;

        for _ in 0..count {
            match &ty.kind {
                TypeKind::Deref(inner) => ty = *inner,
                _ => break,
            }
        }

        UnaliasedType(ty)
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
    pub fn count_leading_derefs(&self) -> usize {
        self.kind.count_leading_derefs()
    }

    pub fn numeric_mode(&self) -> Option<NumericMode> {
        self.kind.numeric_mode()
    }

    pub fn contains_polymorph(&self) -> bool {
        self.kind.contains_polymorph()
    }

    pub fn contains_type_alias(&self) -> bool {
        self.kind.contains_type_alias()
    }

    pub fn display<'a, 'b, 'c>(
        &'a self,
        view: &'b ModuleView<'env>,
        disambiguation: &'c TypeDisplayerDisambiguation<'env>,
    ) -> TypeDisplayer<'a, 'b, 'c, 'env> {
        TypeDisplayer::new(self, view, Cow::Borrowed(disambiguation))
    }

    pub fn display_one<'a, 'b, 'c>(
        &'a self,
        view: &'b ModuleView<'env>,
    ) -> TypeDisplayer<'a, 'b, 'c, 'env> {
        TypeDisplayer::new(
            self,
            view,
            Cow::Owned(TypeDisplayerDisambiguation::single(self)),
        )
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

    pub fn is_integer_like(&self) -> bool {
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
            Self::IntegerLiteralInRange(a, b) => Some(NumericMode::Integer(
                if **a < BigInt::ZERO || **b < BigInt::ZERO {
                    IntegerSign::Signed
                } else {
                    IntegerSign::Unsigned
                },
            )),
            _ => None,
        }
    }

    pub fn count_leading_derefs(&self) -> usize {
        let mut count = 0;
        let mut kind = self;

        loop {
            match kind {
                TypeKind::Deref(inner) => {
                    kind = &inner.kind;
                    count += 1;
                }
                _ => return count,
            }
        }
    }

    pub fn traverse_bfs<T>(&self, initial: T, f: &mut impl FnMut(T, &TypeKind<'env>) -> T) -> T {
        let mut acc = f(initial, self);

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
            | TypeKind::AsciiCharLiteral(_)
            | TypeKind::Void
            | TypeKind::Never
            | TypeKind::Polymorph(_)
            | TypeKind::DirectLabel(_) => acc,
            TypeKind::UserDefined(udt) => {
                for arg in udt.args.iter() {
                    match arg {
                        TypeArg::Type(ty) => acc = ty.kind.traverse_bfs(acc, f),
                        TypeArg::Integer(_) => (),
                    }
                }
                acc
            }
            TypeKind::Ptr(inner) | TypeKind::Deref(inner) | TypeKind::FixedArray(inner, _) => {
                inner.kind.traverse_bfs(acc, f)
            }
        }
    }

    pub fn contains_polymorph(&self) -> bool {
        self.traverse_bfs(false, &mut |acc, kind| {
            acc || matches!(kind, TypeKind::Polymorph(_))
        })
    }

    pub fn contains_type_alias(&self) -> bool {
        self.traverse_bfs(false, &mut |acc, kind| {
            acc || matches!(
                kind,
                TypeKind::UserDefined(UserDefinedType {
                    rest: TypeHeadRest {
                        kind: TypeHeadRestKind::Alias(_),
                        ..
                    },
                    ..
                })
            )
        })
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct UnaliasedUserDefinedType<'env>(pub &'env UserDefinedType<'env>);
