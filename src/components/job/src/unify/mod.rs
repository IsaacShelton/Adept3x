/*
    ====================  components/job/src/unify/mod.rs  ====================
    Definitions for unifying types
    ---------------------------------------------------------------------------
*/

use crate::repr::{Type, TypeKind};
use ast::ConformBehavior;
use data_units::BitUnits;
use itertools::Itertools;
use num_bigint::BigInt;
use primitives::{CInteger, CIntegerAssumptions, FloatSize, IntegerBits, IntegerSign};
use source_files::Source;

pub fn unify_types<'a, 't, 'env: 'a + 't>(
    preferred_type: Option<&Type<'env>>,
    types_iter: impl Iterator<Item = &'a Type<'env>> + Clone,
    behavior: ConformBehavior,
    source: Source,
) -> Option<Type<'env>> {
    let types_iter = types_iter.filter(|ty| !ty.kind.is_never());

    // If all the values have the same type, the unifying type is that type
    if types_iter.clone().all_equal() {
        return Some(
            types_iter
                .clone()
                .next()
                .cloned()
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
        return Some(TypeKind::BitInteger(IntegerBits::Bits32, IntegerSign::Signed).at(source));
    }

    // If all the values are float literals, the unifying type is f64
    if types_iter.clone().all(|ty| ty.kind.is_integer_literal()) {
        return Some(TypeKind::Floating(FloatSize::Bits64).at(source));
    }

    // If all values are integer and floating literals, use the default floating-point type
    if types_iter.clone().all(|ty| {
        matches!(
            ty.kind,
            TypeKind::IntegerLiteral(..) | TypeKind::FloatLiteral(..)
        )
    }) {
        if let Some(TypeKind::Floating(FloatSize::Bits32)) =
            preferred_type.as_ref().map(|ty| &ty.kind)
        {
            return Some(TypeKind::Floating(FloatSize::Bits32).at(source));
        } else {
            return Some(TypeKind::Floating(FloatSize::Bits64).at(source));
        }
    }

    // If all values are integers and integer literals
    if types_iter.clone().all(|ty| ty.kind.is_integer()) {
        return compute_unifying_integer_type(types_iter, behavior, source);
    }

    // If all values are f32's and float literals, the result should be f32
    if types_iter.clone().all(|ty| {
        matches!(
            ty.kind,
            TypeKind::Floating(FloatSize::Bits32) | TypeKind::FloatLiteral(_)
        )
    }) {
        return Some(TypeKind::Floating(FloatSize::Bits32).at(source));
    }

    // Otherwise if all values floating points / integer literals, the result should be f64
    if types_iter.clone().all(|ty| {
        matches!(
            ty.kind,
            TypeKind::Floating(_) | TypeKind::FloatLiteral(_) | TypeKind::IntegerLiteral(_)
        )
    }) {
        return Some(TypeKind::Floating(FloatSize::Bits64).at(source));
    }

    None
}

fn compute_unifying_integer_type<'a, 'env: 'a>(
    types_iter: impl Iterator<Item = &'a Type<'env>>,
    behavior: ConformBehavior,
    source: Source,
) -> Option<Type<'env>> {
    let IntegerProperties {
        largest_loose_used,
        required_bits,
        required_sign,
        ..
    } = IntegerProperties::compute(types_iter, behavior.c_integer_assumptions())?;

    let required_sign = required_sign.unwrap_or(IntegerSign::Signed);
    let assumptions = behavior.c_integer_assumptions();

    if let Some(c_integer) = largest_loose_used {
        let c_integer =
            CInteger::smallest_that_fits(c_integer, required_bits.unwrap(), assumptions)
                .unwrap_or(CInteger::LongLong);

        return Some(TypeKind::CInteger(c_integer, Some(required_sign)).at(source));
    }

    let required_bits = required_bits.unwrap_or(IntegerBits::Bits32);

    return Some(TypeKind::BitInteger(required_bits, required_sign).at(source));
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct IntegerProperties {
    pub largest_loose_used: Option<CInteger>,
    pub required_bits: Option<IntegerBits>,
    pub required_sign: Option<IntegerSign>,
    pub is_concrete: bool,
}

impl IntegerProperties {
    const NONE: Self = Self {
        largest_loose_used: None,
        required_bits: None,
        required_sign: None,
        is_concrete: false,
    };

    pub fn new(ty: &Type, assumptions: CIntegerAssumptions) -> Option<Self> {
        match &ty.kind {
            TypeKind::BitInteger(bits, sign) => Some(Self {
                largest_loose_used: None,
                required_bits: Some(*bits),
                required_sign: Some(*sign),
                is_concrete: true,
            }),
            TypeKind::CInteger(c_integer, sign) => Some(Self {
                largest_loose_used: Some(*c_integer),
                required_bits: Some(c_integer.min_bits(assumptions)),
                required_sign: *sign,
                is_concrete: true,
            }),
            TypeKind::IntegerLiteral(value) => {
                let unsigned_bits = value.bits();

                let sign = (*value < BigInt::ZERO).then_some(IntegerSign::Signed);

                let bits = BitUnits::of(if sign == Some(IntegerSign::Signed) {
                    unsigned_bits + 1
                } else {
                    unsigned_bits
                });

                Some(Self {
                    largest_loose_used: None,
                    required_bits: Some(IntegerBits::new(bits)?),
                    required_sign: sign,
                    is_concrete: false,
                })
            }
            _ => None,
        }
    }

    pub fn compute<'a, 'env: 'a>(
        mut types: impl Iterator<Item = &'a Type<'env>>,
        assumptions: CIntegerAssumptions,
    ) -> Option<IntegerProperties> {
        types.try_fold(IntegerProperties::NONE, |properties, ty| {
            unify_integer_properties(properties, assumptions, ty)
        })
    }
}

pub fn unify_integer_properties(
    a: IntegerProperties,
    assumptions: CIntegerAssumptions,
    ty: &Type,
) -> Option<IntegerProperties> {
    let b = IntegerProperties::new(ty, assumptions)?;

    if a == IntegerProperties::NONE || a == b {
        return Some(b);
    }

    let a_bits = a.required_bits?.bits();
    let b_bits = b.required_bits?.bits();

    let a_sign = a.required_sign;
    let b_sign = b.required_sign;

    if !a.is_concrete || !b.is_concrete {
        let bits = IntegerBits::new(a_bits.max(b_bits).into()).unwrap_or(IntegerBits::Bits64);

        let largest_loose_used =
            CInteger::largest(a.largest_loose_used, b.largest_loose_used).map(|c_integer| {
                CInteger::smallest_that_fits(c_integer, bits, assumptions)
                    .unwrap_or(CInteger::LongLong)
            });

        return Some(IntegerProperties {
            largest_loose_used,
            required_bits: Some(bits),
            required_sign: IntegerSign::strongest(a_sign, b_sign),
            is_concrete: a.is_concrete || b.is_concrete,
        });
    }

    let integer_properties = match (a.largest_loose_used, b.largest_loose_used) {
        (None, None) => {
            // Two normal fixed-size integers

            let a_sign = a_sign.or(b_sign).unwrap();
            let b_sign = b_sign.unwrap_or(a_sign);

            let (bits, sign) = if a_bits >= b_bits && a_sign.is_unsigned() && b_sign.is_signed() {
                (a_bits + BitUnits::of(1), IntegerSign::Signed)
            } else if b_bits >= a_bits && b_sign.is_unsigned() && a_sign.is_signed() {
                (b_bits + BitUnits::of(1), IntegerSign::Signed)
            } else {
                (a_bits.max(b_bits), IntegerSign::stronger(a_sign, b_sign))
            };

            let bits = IntegerBits::new(bits).unwrap_or(IntegerBits::Bits64);

            Some(IntegerProperties {
                largest_loose_used: None,
                required_bits: Some(bits),
                required_sign: Some(sign),
                is_concrete: true,
            })
        }
        (None, Some(min_c_integer)) | (Some(min_c_integer), None) => {
            // One normal fixed-size integer combined with a flexible C integer
            unify_integer_properties_flexible(a, b, min_c_integer, assumptions)
        }
        (Some(a_c_integer), Some(b_c_integer)) => {
            // Two flexible C integers
            unify_integer_properties_flexible(a, b, a_c_integer.max(b_c_integer), assumptions)
        }
    };
    integer_properties
}

fn unify_integer_properties_flexible(
    a: IntegerProperties,
    b: IntegerProperties,
    min_c_integer: CInteger,
    assumptions: CIntegerAssumptions,
) -> Option<IntegerProperties> {
    let a_bits = a.required_bits?.bits();
    let b_bits = b.required_bits?.bits();
    let a_can_be_signed = matches!(a.required_sign, Some(IntegerSign::Signed) | None);
    let b_can_be_signed = matches!(b.required_sign, Some(IntegerSign::Signed) | None);
    let a_can_be_unsigned = matches!(a.required_sign, Some(IntegerSign::Unsigned) | None);
    let b_can_be_unsigned = matches!(b.required_sign, Some(IntegerSign::Unsigned) | None);

    let (bits, maybe_sign) = if a_bits >= b_bits && a_can_be_unsigned && b_can_be_signed {
        (a_bits + BitUnits::of(1), b.required_sign)
    } else if b_bits >= a_bits && b_can_be_unsigned && a_can_be_signed {
        (b_bits + BitUnits::of(1), a.required_sign)
    } else {
        (
            a_bits.max(b_bits),
            IntegerSign::strongest(a.required_sign, b.required_sign),
        )
    };

    let bits = IntegerBits::new(bits)?;

    Some(IntegerProperties {
        largest_loose_used: CInteger::smallest_that_fits(min_c_integer, bits, assumptions),
        required_bits: Some(bits),
        required_sign: maybe_sign,
        is_concrete: true,
    })
}

fn integer_literals_all_fit<'a, 'env: 'a>(
    preferred_type: Option<&Type>,
    mut types: impl Iterator<Item = &'a Type<'env>>,
) -> bool {
    let Some(Type {
        kind: TypeKind::BitInteger(preferred_bits, preferred_sign),
        ..
    }) = preferred_type
    else {
        return false;
    };

    types.all(|ty| match &ty.kind {
        TypeKind::IntegerLiteral(value) => {
            let literal_sign = IntegerSign::from(value);

            let literal_bits = BitUnits::of(match literal_sign {
                IntegerSign::Unsigned => value.bits(),
                IntegerSign::Signed => value.bits() + 1,
            });

            (preferred_sign.is_signed() || literal_sign.is_unsigned())
                && literal_bits <= preferred_bits.bits()
        }
        _ => false,
    })
}
