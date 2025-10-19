#![allow(unused)]

use super::PolyValue;
use crate::repr::{
    Type, TypeArg, TypeHeadRest, TypeHeadRestKind, TypeKind, UnaliasedType, UserDefinedType,
};
use ast_workspace::{TypeAliasRef, TypeDeclRef};
use indexmap::IndexMap;
use std::borrow::Cow;

#[derive(Clone, Debug)]
pub struct TypeMatcher<'env, 'existing> {
    existing: &'existing IndexMap<&'env str, PolyValue<'env>>,
    addition: IndexMap<&'env str, PolyValue<'env>>,
}

impl<'env, 'existing> TypeMatcher<'env, 'existing> {
    pub fn new(existing: &'existing IndexMap<&'env str, PolyValue<'env>>) -> Self {
        Self {
            existing,
            addition: IndexMap::default(),
        }
    }

    pub fn match_type(&mut self, pattern: &'env Type, concrete: &'env Type) -> Result<(), ()> {
        todo!()
    }
}

#[derive(Clone, Debug)]
pub struct WaitingOnTypeAlias {
    type_alias_ref: TypeAliasRef,
}

pub trait OnPolymorph<'env>: FnMut(&str, UnaliasedType<'env>) -> bool {}
impl<'env, F> OnPolymorph<'env> for F where F: FnMut(&str, UnaliasedType<'env>) -> bool {}

pub fn are_types_equal<'env>(
    a_ty: UnaliasedType<'env>,
    b_ty: UnaliasedType<'env>,
    mut on_polymorph: impl OnPolymorph<'env>,
) -> bool {
    let a = &a_ty.0.kind;
    let b = &b_ty.0.kind;

    match a {
        TypeKind::IntegerLiteral(a_value) => {
            if let TypeKind::IntegerLiteral(b_value) = b {
                a_value == b_value
            } else {
                false
            }
        }
        TypeKind::IntegerLiteralInRange(a_min, a_max) => {
            if let TypeKind::IntegerLiteralInRange(b_min, b_max) = b {
                a_min == b_min && a_max == b_max
            } else {
                false
            }
        }
        TypeKind::FloatLiteral(a_value) => {
            if let TypeKind::FloatLiteral(b_value) = b {
                a_value == b_value
            } else {
                false
            }
        }
        TypeKind::NullLiteral => matches!(b, TypeKind::NullLiteral),
        TypeKind::BooleanLiteral(a_value) => {
            if let TypeKind::BooleanLiteral(b_value) = b {
                a_value == b_value
            } else {
                false
            }
        }
        TypeKind::AsciiCharLiteral(a_value) => {
            if let TypeKind::AsciiCharLiteral(b_value) = b {
                a_value == b_value
            } else {
                false
            }
        }
        TypeKind::Boolean => matches!(b, TypeKind::Boolean),
        TypeKind::BitInteger(a_bits, a_sign) => {
            if let TypeKind::BitInteger(b_bits, b_sign) = b {
                a_bits == b_bits && a_sign == b_sign
            } else {
                false
            }
        }
        TypeKind::CInteger(a_int, a_sign) => {
            if let TypeKind::CInteger(b_int, b_sign) = b {
                a_int == b_int && a_sign == b_sign
            } else {
                false
            }
        }
        TypeKind::SizeInteger(a_sign) => {
            if let TypeKind::SizeInteger(b_sign) = b {
                a_sign == b_sign
            } else {
                false
            }
        }
        TypeKind::Floating(a_size) => {
            if let TypeKind::Floating(b_size) = b {
                a_size == b_size
            } else {
                false
            }
        }
        TypeKind::Ptr(a_inner) => {
            if let TypeKind::Ptr(b_inner) = b {
                are_types_equal(UnaliasedType(a_inner), UnaliasedType(b_inner), on_polymorph)
            } else {
                false
            }
        }
        TypeKind::Deref(a_inner) => {
            if let TypeKind::Deref(b_inner) = b {
                are_types_equal(UnaliasedType(a_inner), UnaliasedType(b_inner), on_polymorph)
            } else {
                false
            }
        }
        TypeKind::Void => matches!(b, TypeKind::Void),
        TypeKind::Never => matches!(b, TypeKind::Never),
        TypeKind::FixedArray(a_inner, a_size) => {
            if let TypeKind::FixedArray(b_inner, b_size) = b {
                a_size == b_size
                    && are_types_equal(UnaliasedType(a_inner), UnaliasedType(b_inner), on_polymorph)
            } else {
                false
            }
        }
        // Type aliases were already handled
        TypeKind::UserDefined(UserDefinedType {
            rest:
                TypeHeadRest {
                    kind: TypeHeadRestKind::Alias(..),
                    ..
                },
            args: a_args,
            ..
        }) => {
            unreachable!()
        }
        // Non-aliases
        TypeKind::UserDefined(
            a_udt @ UserDefinedType {
                rest: TypeHeadRest { kind: a_kind, .. },
                ..
            },
        ) => {
            if let TypeKind::UserDefined(
                b_udt @ UserDefinedType {
                    rest: TypeHeadRest { kind: b_kind, .. },
                    ..
                },
            ) = b
            {
                // The type alias cases should have already been handled
                assert!(!a_kind.is_alias());
                assert!(!b_kind.is_alias());

                a_kind == b_kind && are_type_arg_lists_equal(a_udt.args, b_udt.args)
            } else {
                false
            }
        }
        TypeKind::Polymorph(a_name) => {
            if let TypeKind::Polymorph(b_name) = b {
                a_name == b_name || on_polymorph(a_name, b_ty)
            } else {
                on_polymorph(a_name, b_ty)
            }
        }
        TypeKind::DirectLabel(a_label) => {
            if let TypeKind::DirectLabel(b_label) = b {
                a_label == b_label
            } else {
                false
            }
        }
    }
}

pub fn are_type_arg_lists_equal<'env>(a_args: &[TypeArg<'env>], b_args: &[TypeArg<'env>]) -> bool {
    if a_args.len() != b_args.len() {
        return false;
    }

    for (a_arg, b_arg) in a_args.iter().zip(b_args) {
        if !are_type_args_equal(a_arg, b_arg) {
            return false;
        }
    }

    true
}

pub fn are_type_args_equal<'env>(a_arg: &TypeArg<'env>, b_arg: &TypeArg<'env>) -> bool {
    match a_arg {
        TypeArg::Type(_) => todo!(),
        TypeArg::Integer(big_int) => todo!(),
    }
    todo!()
}
