use super::{conform_expr, ConformMode};
use crate::{
    ast::{ConformBehavior, Source},
    resolved::{self, FloatSize, IntegerBits, IntegerSign, TypedExpr},
};
use itertools::Itertools;
use num_bigint::BigInt;
use num_traits::Zero;
use std::borrow::Borrow;

pub fn unify_types(
    preferred_type: Option<&resolved::Type>,
    exprs: &mut [&mut TypedExpr],
    conform_behavior: ConformBehavior,
    conform_source: Source,
) -> Option<resolved::Type> {
    let unified_type = unifying_type_for(preferred_type, exprs, conform_behavior, conform_source);

    if let Some(unified_type) = &unified_type {
        for expr in exprs.iter_mut() {
            **expr = match conform_expr(
                expr,
                unified_type,
                ConformMode::Normal,
                ConformBehavior::Adept,
                conform_source,
            ) {
                Some(conformed) => conformed,
                None => {
                    panic!(
                        "cannot conform to unified type {unified_type} for value of type {}",
                        expr.resolved_type,
                    );
                }
            }
        }
    }

    unified_type
}

fn unify_integer_properties(
    required_bits: Option<IntegerBits>,
    required_sign: Option<IntegerSign>,
    ty: &resolved::Type,
) -> Option<(Option<IntegerBits>, Option<IntegerSign>)> {
    let (new_bits, new_sign) = match &ty.kind {
        resolved::TypeKind::Integer { bits, sign } => (
            match required_sign {
                Some(IntegerSign::Unsigned) if *sign == IntegerSign::Signed => {
                    // Compensate for situations like i32 + u32
                    bits.bits() as u64 + 1
                }
                _ => bits.bits() as u64,
            },
            Some(*sign),
        ),
        resolved::TypeKind::IntegerLiteral(value) => {
            let unsigned_bits = value.bits();

            let (bits, sign) = if *value < BigInt::zero() {
                (unsigned_bits + 1, Some(IntegerSign::Signed))
            } else {
                (unsigned_bits, None)
            };

            (bits, sign)
        }
        _ => return None,
    };

    let check_overflow = match ty.kind {
        resolved::TypeKind::Integer {
            bits: IntegerBits::Normal,
            ..
        } => true,
        _ => required_bits == Some(IntegerBits::Normal),
    };

    let old_bits = match (required_sign, new_sign) {
        (Some(IntegerSign::Signed), Some(IntegerSign::Unsigned)) => {
            required_bits.map(|bits| bits.bits() + 1).unwrap_or(0)
        }
        _ => required_bits.map(|bits| bits.bits()).unwrap_or(0),
    };
    let old_sign = required_sign;

    let sign_kind = match (old_sign, new_sign) {
        (Some(old_sign), Some(new_sign)) => {
            if old_sign == IntegerSign::Signed || new_sign == IntegerSign::Signed {
                Some(IntegerSign::Signed)
            } else {
                Some(IntegerSign::Unsigned)
            }
        }
        (Some(old_sign), None) => Some(old_sign),
        (None, Some(new_sign)) => Some(new_sign),
        (None, None) => None,
    };

    let bits_kind = IntegerBits::new(new_bits.max(old_bits.into())).map(|bits| match bits {
        IntegerBits::Bits64 => {
            if check_overflow {
                IntegerBits::Normal
            } else {
                bits
            }
        }
        _ => bits,
    });

    bits_kind.map(|bits_kind| (Some(bits_kind), sign_kind))
}

fn bits_and_sign_for(
    types: &[&resolved::Type],
) -> Option<(Option<IntegerBits>, Option<IntegerSign>)> {
    types
        .iter()
        .try_fold((None, None), |(maybe_bits, maybe_sign), ty| {
            unify_integer_properties(maybe_bits, maybe_sign, ty)
        })
}

fn do_integer_literal_types_fit_in_integer(
    preferred_bits: IntegerBits,
    preferred_sign: IntegerSign,
    types: &[&resolved::Type],
) -> bool {
    types
        .iter()
        .map(|resolved_type| match &resolved_type.kind {
            resolved::TypeKind::IntegerLiteral(value) => value,
            _ => panic!("expected integer literal type"),
        })
        .all(|value| {
            let sign = if *value < BigInt::zero() {
                IntegerSign::Signed
            } else {
                IntegerSign::Unsigned
            };

            let bits = match sign {
                IntegerSign::Unsigned => value.bits(),
                IntegerSign::Signed => value.bits() + 1,
            };

            bits <= preferred_bits.bits().into()
                && (preferred_sign != IntegerSign::Unsigned || sign == IntegerSign::Unsigned)
        })
}

fn unifying_type_for(
    preferred_type: Option<&resolved::Type>,
    exprs: &[impl Borrow<TypedExpr>],
    _conform_behavior: ConformBehavior,
    source: Source,
) -> Option<resolved::Type> {
    let types = exprs
        .iter()
        .map(|expr| &expr.borrow().resolved_type)
        .collect_vec();

    if types.iter().all_equal() {
        return Some(
            exprs
                .first()
                .map(|expr| expr.borrow().resolved_type.clone())
                .unwrap_or_else(|| resolved::TypeKind::Void.at(source)),
        );
    }

    // If all integer literals
    if types
        .iter()
        .all(|resolved_type| matches!(resolved_type.kind, resolved::TypeKind::IntegerLiteral(..)))
    {
        return Some(match preferred_type.map(|ty| &ty.kind) {
            Some(resolved::TypeKind::Integer {
                bits: preferred_bits,
                sign: preferred_sign,
            }) if do_integer_literal_types_fit_in_integer(
                *preferred_bits,
                *preferred_sign,
                &types[..],
            ) =>
            {
                preferred_type.unwrap().clone()
            }
            _ => resolved::TypeKind::Integer {
                bits: IntegerBits::Normal,
                sign: IntegerSign::Signed,
            }
            .at(source),
        });
    }

    // If all (integer/float) literals
    if types.iter().all(|resolved_type| {
        matches!(
            resolved_type.kind,
            resolved::TypeKind::IntegerLiteral(..) | resolved::TypeKind::FloatLiteral(..)
        )
    }) {
        return Some(resolved::TypeKind::Float(FloatSize::Normal).at(source));
    }

    // If all integers and integer literals
    if types.iter().all(|resolved_type| {
        matches!(
            resolved_type.kind,
            resolved::TypeKind::IntegerLiteral(..) | resolved::TypeKind::Integer { .. }
        )
    }) {
        let (bits, sign) = bits_and_sign_for(&types[..])?;

        let bits = bits.unwrap_or(IntegerBits::Normal);
        let sign = sign.unwrap_or(IntegerSign::Signed);

        return Some(resolved::TypeKind::Integer { bits, sign }.at(source));
    }

    None
}
