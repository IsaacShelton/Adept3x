use super::integer_literals::integer_literals_all_fit;
use crate::{
    ast::{CInteger, ConformBehavior, FloatSize, IntegerBits},
    resolved::{IntegerSign, Type, TypeKind, TypedExpr},
    source_files::Source,
};
use itertools::Itertools;
use std::borrow::Borrow;

pub fn compute_unifying_type(
    preferred_type: Option<&Type>,
    values: &[impl Borrow<TypedExpr>],
    _conform_behavior: ConformBehavior,
    source: Source,
) -> Option<Type> {
    let types_iter = values.iter().map(|expr| &expr.borrow().resolved_type);

    // If all the values have the same type, the unifying type is that type
    if types_iter.clone().all_equal() {
        return Some(
            values
                .first()
                .map(|expr| expr.borrow().resolved_type.clone())
                .unwrap_or_else(|| TypeKind::Void.at(source)),
        );
    }

    // If all the values are integer literals, the unifying type is either
    // the preferred type or the default integer type
    if types_iter.clone().all(|ty| ty.kind.is_integer_literal()) {
        // If the preferred type is an integer type that can fit them, use the preferred type
        if integer_literals_all_fit(preferred_type, types_iter) {
            return Some(preferred_type.unwrap().clone());
        }

        // Otherwise, use the default integer type
        return Some(TypeKind::Integer(IntegerBits::Bits32, IntegerSign::Signed).at(source));
    }

    // If all values are integer and floating literals, use the default floating-point type
    // NOTE: TODO: Handle case when `f32` is the preferred type?
    if types_iter.clone().all(|resolved_type| {
        matches!(
            resolved_type.kind,
            TypeKind::IntegerLiteral(..) | TypeKind::FloatLiteral(..)
        )
    }) {
        return Some(TypeKind::Floating(FloatSize::Bits64).at(source));
    }

    // If all values are integers and integer literals
    if types_iter.clone().all(|ty| ty.kind.is_integer_like()) {
        return compute_unifying_integer_type(types_iter, source);
    }

    None
}

fn compute_unifying_integer_type<'a>(
    types_iter: impl Iterator<Item = &'a Type>,
    source: Source,
) -> Option<Type> {
    let IntegerProperties {
        largest_loose_used,
        required_bits,
        required_sign,
    } = IntegerProperties::compute(types_iter)?;

    let required_sign = required_sign.unwrap_or(IntegerSign::Signed);

    if let Some(c_integer) = largest_loose_used {
        let c_integer = CInteger::smallest_that_fits(c_integer, required_bits.unwrap())
            .unwrap_or(CInteger::LongLong);

        return Some(TypeKind::CInteger(c_integer, Some(required_sign)).at(source));
    }

    let required_bits = required_bits.unwrap_or(IntegerBits::Bits32);

    return Some(TypeKind::Integer(required_bits, required_sign).at(source));
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct IntegerProperties {
    pub largest_loose_used: Option<CInteger>,
    pub required_bits: Option<IntegerBits>,
    pub required_sign: Option<IntegerSign>,
}

impl IntegerProperties {
    const NONE: Self = Self {
        largest_loose_used: None,
        required_bits: None,
        required_sign: None,
    };

    pub fn new(ty: &Type) -> Option<Self> {
        match &ty.kind {
            TypeKind::Integer(bits, sign) => Some(Self {
                largest_loose_used: None,
                required_bits: Some(*bits),
                required_sign: Some(*sign),
            }),
            TypeKind::CInteger(c_integer, sign) => Some(Self {
                largest_loose_used: Some(*c_integer),
                required_bits: Some(c_integer.min_bits()),
                required_sign: *sign,
            }),
            TypeKind::IntegerLiteral(value) => {
                let unsigned_bits = value.bits();

                let sign = IntegerSign::from(value);

                let bits = sign
                    .is_signed()
                    .then_some(unsigned_bits + 1)
                    .unwrap_or(unsigned_bits);

                Some(Self {
                    largest_loose_used: None,
                    required_bits: Some(IntegerBits::new(bits)?),
                    required_sign: Some(sign),
                })
            }
            _ => None,
        }
    }

    pub fn compute<'a>(mut types: impl Iterator<Item = &'a Type>) -> Option<IntegerProperties> {
        types.try_fold(IntegerProperties::NONE, unify_integer_properties)
    }
}

pub fn unify_integer_properties(a: IntegerProperties, ty: &Type) -> Option<IntegerProperties> {
    let b = IntegerProperties::new(ty)?;

    if a == b {
        return Some(a);
    }

    let integer_properties = match (a.largest_loose_used, b.largest_loose_used) {
        (None, None) => {
            // Two normal fixed-size integers

            let a_bits = a.required_bits?.bits();
            let b_bits = b.required_bits?.bits();
            let a_sign = a.required_sign?;
            let b_sign = b.required_sign?;

            let (bits, sign) = if a_bits >= b_bits && a_sign.is_unsigned() && b_sign.is_signed() {
                (a_bits + 1, IntegerSign::Signed)
            } else if b_bits >= a_bits && b_sign.is_unsigned() && a_sign.is_signed() {
                (a_bits + 1, IntegerSign::Signed)
            } else {
                let sign = (a_sign.is_signed() || b_sign.is_signed())
                    .then_some(IntegerSign::Signed)
                    .unwrap_or(IntegerSign::Unsigned);

                (a_bits.max(b_bits), sign)
            };

            let bits = IntegerBits::new(bits.into()).unwrap_or(IntegerBits::Bits64);

            Some(IntegerProperties {
                largest_loose_used: None,
                required_bits: Some(bits),
                required_sign: Some(sign),
            })
        }
        (None, Some(min_c_integer)) | (Some(min_c_integer), None) => {
            // One normal fixed-size integer combined with a flexible C integer
            unify_integer_properties_flexible(a, b, min_c_integer)
        }
        (Some(a_c_integer), Some(b_c_integer)) => {
            // Two flexible C integers
            unify_integer_properties_flexible(a, b, a_c_integer.max(b_c_integer))
        }
    };
    integer_properties
}

fn unify_integer_properties_flexible(
    a: IntegerProperties,
    b: IntegerProperties,
    min_c_integer: CInteger,
) -> Option<IntegerProperties> {
    let a_bits = a.required_bits?.bits();
    let b_bits = b.required_bits?.bits();
    let a_can_be_signed = matches!(a.required_sign, Some(IntegerSign::Signed) | None);
    let b_can_be_signed = matches!(b.required_sign, Some(IntegerSign::Signed) | None);
    let a_can_be_unsigned = matches!(a.required_sign, Some(IntegerSign::Unsigned) | None);
    let b_can_be_unsigned = matches!(b.required_sign, Some(IntegerSign::Unsigned) | None);

    let (bits, maybe_sign) = if a_bits >= b_bits && a_can_be_unsigned && b_can_be_signed {
        (a_bits + 1, b.required_sign)
    } else if b_bits >= a_bits && b_can_be_unsigned && a_can_be_signed {
        (a_bits + 1, a.required_sign)
    } else {
        let sign = match (a.required_sign, b.required_sign) {
            (None, None) => None,
            (None, Some(sign)) | (Some(sign), None) => {
                sign.is_signed().then_some(IntegerSign::Signed)
            }
            (Some(a_sign), Some(b_sign)) => Some(
                (a_sign.is_signed() || b_sign.is_signed())
                    .then_some(IntegerSign::Signed)
                    .unwrap_or(IntegerSign::Unsigned),
            ),
        };

        (a_bits.max(b_bits), sign)
    };

    let bits = IntegerBits::new(bits.into()).unwrap_or(IntegerBits::Bits64);

    Some(IntegerProperties {
        largest_loose_used: CInteger::smallest_that_fits(min_c_integer, bits),
        required_bits: Some(bits),
        required_sign: maybe_sign,
    })
}
