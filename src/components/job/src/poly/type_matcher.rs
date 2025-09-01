#![allow(unused)]

use super::PolyValue;
use crate::repr::{Type, TypeArg, TypeKind, UserDefinedType};
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

pub fn are_types_equal<'env>(
    a: &TypeKind<'env>,
    b: &TypeKind<'env>,
    mut on_polymorph: impl FnMut(&str, &TypeKind<'env>) -> Result<bool, WaitingOnTypeAlias>,
    on_resolve_type_alias: impl Fn(TypeAliasRef, &[TypeArg<'env>]) -> Result<TypeKind<'env>, ()>,
) -> Result<bool, WaitingOnTypeAlias> {
    // Resolve outer type alias layers of `b`
    let mut b = Cow::Borrowed(b);
    let b = loop {
        let TypeKind::UserDefined(udt) = b.as_ref() else {
            break b;
        };

        let type_alias_ref = match udt.type_decl_ref {
            ast_workspace::TypeDeclRef::Alias(type_alias_ref) => type_alias_ref,
            _ => break b,
        };

        b = Cow::Owned(
            on_resolve_type_alias(type_alias_ref, udt.args)
                .map_err(|_| WaitingOnTypeAlias { type_alias_ref })?,
        );
    };
    let b = b.as_ref();

    match a {
        TypeKind::IntegerLiteral(a_value) => Ok(if let TypeKind::IntegerLiteral(b_value) = b {
            a_value == b_value
        } else {
            false
        }),
        TypeKind::FloatLiteral(a_value) => Ok(if let TypeKind::FloatLiteral(b_value) = b {
            a_value == b_value
        } else {
            false
        }),
        TypeKind::NullLiteral => Ok(matches!(b, TypeKind::NullLiteral)),
        TypeKind::BooleanLiteral(a_value) => Ok(if let TypeKind::BooleanLiteral(b_value) = b {
            a_value == b_value
        } else {
            false
        }),
        TypeKind::Boolean => Ok(matches!(b, TypeKind::Boolean)),
        TypeKind::BitInteger(a_bits, a_sign) => {
            Ok(if let TypeKind::BitInteger(b_bits, b_sign) = b {
                a_bits == b_bits && a_sign == b_sign
            } else {
                false
            })
        }
        TypeKind::CInteger(a_int, a_sign) => Ok(if let TypeKind::CInteger(b_int, b_sign) = b {
            a_int == b_int && a_sign == b_sign
        } else {
            false
        }),
        TypeKind::SizeInteger(a_sign) => Ok(if let TypeKind::SizeInteger(b_sign) = b {
            a_sign == b_sign
        } else {
            false
        }),
        TypeKind::Floating(a_size) => Ok(if let TypeKind::Floating(b_size) = b {
            a_size == b_size
        } else {
            false
        }),
        TypeKind::Ptr(a_inner) => Ok(if let TypeKind::Ptr(b_inner) = b {
            are_types_equal(
                &a_inner.kind,
                &b_inner.kind,
                on_polymorph,
                on_resolve_type_alias,
            )?
        } else {
            false
        }),
        TypeKind::Void => Ok(matches!(b, TypeKind::Void)),
        TypeKind::Never => Ok(matches!(b, TypeKind::Never)),
        TypeKind::FixedArray(a_inner, a_size) => {
            Ok(if let TypeKind::FixedArray(b_inner, b_size) = b {
                a_size == b_size
                    && are_types_equal(
                        &a_inner.kind,
                        &b_inner.kind,
                        on_polymorph,
                        on_resolve_type_alias,
                    )?
            } else {
                false
            })
        }
        TypeKind::UserDefined(UserDefinedType {
            type_decl_ref: TypeDeclRef::Alias(a_type_alias_ref),
            args: a_args,
            ..
        }) => {
            let new_a = on_resolve_type_alias(*a_type_alias_ref, a_args).map_err(|_| {
                WaitingOnTypeAlias {
                    type_alias_ref: *a_type_alias_ref,
                }
            })?;

            // TODO: We shouldn't be doing unbounded recursion like this
            are_types_equal(&new_a, b, on_polymorph, on_resolve_type_alias)
        }
        // Non-aliases
        TypeKind::UserDefined(
            a_udt @ UserDefinedType {
                type_decl_ref: a_type_decl_ref,
                ..
            },
        ) => Ok(
            if let TypeKind::UserDefined(
                b_udt @ UserDefinedType {
                    type_decl_ref: b_type_decl_ref,
                    ..
                },
            ) = b
            {
                // The type alias cases should have already been handled
                assert!(!matches!(a_type_decl_ref, TypeDeclRef::Alias(_)));
                assert!(!matches!(b_type_decl_ref, TypeDeclRef::Alias(_)));

                // NOTE: I think we need to resolve the actual aliases
                // in order to have better testing for if they match,
                // since for example,
                // typealias X<$T> = int
                //
                // would be the same no matter what $T is,
                // and this is something that could happen in higher abstractions.

                a_type_decl_ref == b_type_decl_ref
                    && are_type_arg_lists_equal(a_udt.args, b_udt.args)?
            } else {
                false
            },
        ),
        TypeKind::Polymorph(name) => on_polymorph(name, b),
        TypeKind::DirectLabel(a_label) => Ok(if let TypeKind::DirectLabel(b_label) = b {
            a_label == b_label
        } else {
            false
        }),
    }
}

pub fn are_type_arg_lists_equal<'env>(
    a_args: &[TypeArg<'env>],
    b_args: &[TypeArg<'env>],
) -> Result<bool, WaitingOnTypeAlias> {
    if a_args.len() != b_args.len() {
        return Ok(false);
    }

    for (a_arg, b_arg) in a_args.iter().zip(b_args) {
        if !are_type_args_equal(a_arg, b_arg)? {
            return Ok(false);
        }
    }

    Ok(true)
}

pub fn are_type_args_equal<'env>(
    a_arg: &TypeArg<'env>,
    b_arg: &TypeArg<'env>,
) -> Result<bool, WaitingOnTypeAlias> {
    match a_arg {
        TypeArg::Type(_) => todo!(),
        TypeArg::Integer(big_int) => todo!(),
    }
    todo!()
}
