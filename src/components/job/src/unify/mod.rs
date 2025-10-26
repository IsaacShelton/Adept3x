/*
    ====================  components/job/src/unify/mod.rs  ====================
    Definitions for unifying types
    ---------------------------------------------------------------------------
*/

use crate::{
    BuiltinTypes, ExecutionCtx,
    conform::{ConformMode, UnaryCast, conform_to},
    module_graph::ModuleView,
    repr::{Type, TypeDisplayerDisambiguation, TypeKind, UnaliasedType},
};
use ast::ConformBehavior;
use data_units::BitUnits;
use diagnostics::ErrorDiagnostic;
use itertools::Itertools;
use num_bigint::BigInt;
use primitives::{
    CInteger, CIntegerAssumptions, FloatSize, IntegerBits, IntegerSign, OptionIntegerSignExt,
};
use source_files::Source;

#[derive(Debug)]
pub struct UnifySolution<'env, T> {
    pub unified: UnaliasedType<'env>,
    pub unary_casts: Vec<(T, UnaryCast<'env>)>,
}

pub fn unify_types<'env, T>(
    ctx: &mut ExecutionCtx<'env>,
    types_iter: impl Iterator<Item = (T, UnaliasedType<'env>)> + Clone,
    behavior: ConformBehavior,
    builtin_types: &'env BuiltinTypes<'env>,
    view: &'env ModuleView<'env>,
    source: Source,
) -> Result<Option<UnifySolution<'env, T>>, ErrorDiagnostic> {
    let target = view.target();

    let Some(to_ty) = unify_types_find_solution(
        ctx,
        types_iter.clone().map(|(_id, ty)| ty),
        behavior,
        builtin_types,
        source,
    )?
    else {
        return Ok(None);
    };

    let mut casts = Vec::with_capacity(types_iter.clone().count());

    for (id, from_ty) in types_iter {
        if from_ty.0.kind.is_never() {
            continue;
        }

        let Some(conformed) = conform_to(
            ctx,
            from_ty,
            to_ty,
            behavior.c_integer_assumptions(),
            builtin_types,
            target,
            ConformMode::Explicit,
            // NOTE: We don't handle polymorphs, since this is only for trivial
            // type unification.
            |_, _| false,
            source,
        ) else {
            let disambiguation = TypeDisplayerDisambiguation::new([from_ty.0, to_ty.0].into_iter());

            return Err(ErrorDiagnostic::ice(
                format!(
                    "Failed to conform value of {} to calculated unifying type {}",
                    from_ty.display(view, &disambiguation),
                    to_ty.display(view, &disambiguation)
                ),
                Some(source),
            ));
        };

        if let Some(cast) = conformed.cast {
            casts.push((id, cast));
        }
    }

    Ok(Some(UnifySolution {
        unified: to_ty,
        unary_casts: casts,
    }))
}

pub fn unify_types_find_solution<'env>(
    ctx: &mut ExecutionCtx<'env>,
    types_iter: impl Iterator<Item = UnaliasedType<'env>> + Clone,
    behavior: ConformBehavior,
    builtin_types: &'env BuiltinTypes<'env>,
    source: Source,
) -> Result<Option<UnaliasedType<'env>>, ErrorDiagnostic> {
    let (min, max) = types_iter.clone().fold((0, 0), |(min, max), ty| {
        let has = ty.0.count_leading_derefs();
        (min.min(has), max.max(has))
    });
    let derefs_to_remove = max - min;

    let types_iter = types_iter.map(|ty| ty.without_leading_derefs(derefs_to_remove));
    let incoming = types_iter.filter(|ty| !ty.0.kind.is_never());

    // If unreachable, the unifying type is the never type
    if incoming.clone().next().is_none() {
        return Ok(Some(builtin_types.never()));
    }

    // If all the values have the same type, the unifying type is that type
    if incoming.clone().all_equal() {
        return Ok(Some(
            incoming
                .clone()
                .next()
                .unwrap_or_else(|| builtin_types.void()),
        ));
    }

    // If all the values are integer literals, the unifying type is either
    // the preferred type or the default integer type
    if incoming
        .clone()
        .all(|ty| ty.0.kind.is_integer_literal() || ty.0.kind.is_integer_literal_in_range())
    {
        let mut min = Option::<&'env BigInt>::None;
        let mut max = Option::<&'env BigInt>::None;

        for ty in incoming {
            match &ty.0.kind {
                TypeKind::IntegerLiteral(big_int) => {
                    if let Some(current_min) = min {
                        if *big_int < current_min {
                            min = Some(*big_int);
                        }
                    } else {
                        min = Some(big_int);
                    }
                    if let Some(current_max) = max {
                        if *big_int > current_max {
                            max = Some(*big_int);
                        }
                    } else {
                        max = Some(big_int);
                    }
                }
                TypeKind::IntegerLiteralInRange(range_min, range_max) => {
                    if let Some(current_min) = min {
                        if *range_min < current_min {
                            min = Some(*range_min);
                        }
                    } else {
                        min = Some(range_min);
                    }
                    if let Some(current_max) = max {
                        if *range_max > current_max {
                            max = Some(*range_max);
                        }
                    } else {
                        max = Some(range_max);
                    }
                }
                _ => unreachable!(),
            }
        }

        // We're guarenteed to have these, because of the check for "never" done earlier
        let (min, max) = min.zip(max).unwrap();

        return Ok(Some(UnaliasedType(
            ctx.alloc(
                if min == max {
                    TypeKind::IntegerLiteral(min)
                } else {
                    TypeKind::IntegerLiteralInRange(min, max)
                }
                .at(source),
            ),
        )));
    }

    // If all the values are float literals, the unifying type is f64
    if incoming.clone().all(|ty| ty.0.kind.is_integer_literal()) {
        return Ok(Some(builtin_types.f64()));
    }

    // If all values are integer and floating literals, use the default floating-point type
    if incoming.clone().all(|ty| {
        matches!(
            ty.0.kind,
            TypeKind::IntegerLiteral(..) | TypeKind::FloatLiteral(..)
        )
    }) {
        return Ok(Some(builtin_types.f64()));
    }

    // If all values are integers and integer literals
    if incoming.clone().all(|ty| ty.0.kind.is_integer_like()) {
        return compute_unifying_integer_type(ctx, incoming, behavior, source);
    }

    // If all values are f32's and float literals, the result should be f32
    if incoming.clone().all(|ty| {
        matches!(
            ty.0.kind,
            TypeKind::Floating(FloatSize::Bits32) | TypeKind::FloatLiteral(_)
        )
    }) {
        return Ok(Some(builtin_types.f32()));
    }

    // Otherwise if all values floating points / integer literals, the result should be f64
    if incoming.clone().all(|ty| {
        matches!(
            ty.0.kind,
            TypeKind::Floating(_) | TypeKind::FloatLiteral(_) | TypeKind::IntegerLiteral(_)
        )
    }) {
        return Ok(Some(builtin_types.f64()));
    }

    Ok(None)
}

fn compute_unifying_integer_type<'env>(
    ctx: &mut ExecutionCtx<'env>,
    types_iter: impl Iterator<Item = UnaliasedType<'env>>,
    behavior: ConformBehavior,
    source: Source,
) -> Result<Option<UnaliasedType<'env>>, ErrorDiagnostic> {
    let Some(IntegerProperties {
        largest_loose_used,
        required_bits,
        required_sign,
        ..
    }) = IntegerProperties::compute(types_iter, behavior.c_integer_assumptions())
    else {
        return Ok(None);
    };

    let required_sign = required_sign.unwrap_or(IntegerSign::Signed);
    let assumptions = behavior.c_integer_assumptions();

    if let Some(c_integer) = largest_loose_used {
        let c_integer =
            CInteger::smallest_that_fits(c_integer, required_bits.unwrap(), assumptions)
                .unwrap_or(CInteger::LongLong);

        return Ok(Some(UnaliasedType(ctx.alloc(
            TypeKind::CInteger(c_integer, Some(required_sign)).at(source),
        ))));
    }

    let required_bits = required_bits.unwrap_or(IntegerBits::Bits32);

    return Ok(Some(UnaliasedType(ctx.alloc(
        TypeKind::BitInteger(required_bits, required_sign).at(source),
    ))));
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

                let sign = (**value < BigInt::ZERO).then_some(IntegerSign::Signed);

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

    pub fn compute<'env>(
        mut types: impl Iterator<Item = UnaliasedType<'env>>,
        assumptions: CIntegerAssumptions,
    ) -> Option<IntegerProperties> {
        types.try_fold(IntegerProperties::NONE, |properties, ty| {
            unify_integer_properties(properties, assumptions, ty.0)
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
        let bits = if a_bits == b_bits {
            if a_sign.is_signed() != b_sign.is_signed() {
                a_bits + BitUnits::of(1)
            } else {
                a_bits
            }
        } else if a_bits > b_bits {
            if a_sign.is_signed() && !b_sign.is_signed() {
                a_bits + BitUnits::of(1)
            } else {
                a_bits
            }
        } else {
            if b_sign.is_signed() && !a_sign.is_signed() {
                b_bits + BitUnits::of(1)
            } else {
                b_bits
            }
        };

        let bits = IntegerBits::new(bits).unwrap_or(IntegerBits::Bits64);

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
