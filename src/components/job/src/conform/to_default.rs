use crate::{
    BuiltinTypes, ExecutionCtx,
    conform::{UnaryCast, does_integer_literal_fit, does_integer_literal_fit_in_c},
    repr::{TypeKind, UnaliasedType},
};
use diagnostics::ErrorDiagnostic;
use num_bigint::BigInt;
use num_traits::ToPrimitive;
use ordered_float::NotNan;
use primitives::{CIntegerAssumptions, IntegerSign};
use source_files::Source;
use target::Target;

#[derive(Debug)]
pub struct Conform<'env> {
    pub ty: UnaliasedType<'env>,
    pub cast: Option<UnaryCast<'env>>,
}

impl<'env> Conform<'env> {
    pub fn new(ty: UnaliasedType<'env>, cast: UnaryCast<'env>) -> Self {
        Self {
            ty,
            cast: Some(cast),
        }
    }

    pub fn identity(ty: UnaliasedType<'env>) -> Self {
        Self { ty, cast: None }
    }

    pub fn after_implicit_dereferences(
        self,
        ctx: &mut ExecutionCtx<'env>,
        mut from_ty: UnaliasedType<'env>,
        mut count: usize,
    ) -> Self {
        let mut result: Conform<'env> = self;

        while count > 0 {
            let TypeKind::Deref(after_deref) = &from_ty.0.kind else {
                panic!("cannot implicitly dereference non deref'$T type");
            };

            result = Self::new(
                from_ty,
                UnaryCast::Dereference {
                    after_deref: UnaliasedType(*after_deref),
                    then: std::mem::take(&mut result.cast).map(|cast| &*ctx.alloc(cast)),
                },
            );

            from_ty = UnaliasedType(after_deref);
            count -= 1;
        }

        result
    }
}

pub fn conform_to_default<'env>(
    ctx: &mut ExecutionCtx<'env>,
    original_from_ty: UnaliasedType<'env>,
    assumptions: CIntegerAssumptions,
    builtin_types: &'env BuiltinTypes<'env>,
    target: &Target,
) -> Result<Conform<'env>, ErrorDiagnostic> {
    let source = original_from_ty.0.source;

    let default_integer_types = [
        &builtin_types.i32,
        &builtin_types.u32,
        &builtin_types.i64,
        &builtin_types.u64,
    ];

    let mut ty = original_from_ty;
    let mut dereferences = 0;

    loop {
        match &ty.0.kind {
            TypeKind::Deref(inner_type) => {
                ty = UnaliasedType(inner_type);
                dereferences += 1;
            }
            _ => break,
        }
    }

    let inner_conform = match &ty.0.kind {
        TypeKind::IntegerLiteral(big_int) => default_integer_types
            .into_iter()
            .flat_map(|possible_type| {
                default_from_integer_literal(
                    ctx,
                    big_int,
                    assumptions,
                    source,
                    &possible_type.kind,
                    builtin_types,
                    target,
                )
            })
            .next()
            .ok_or_else(|| {
                ErrorDiagnostic::new(
                    "Integer is too large to represent without concrete type",
                    source,
                )
            })?,
        TypeKind::FloatLiteral(not_nan) => Conform::new(
            builtin_types.f64(),
            UnaryCast::SpecializeFloat(not_nan.clone()),
        ),
        TypeKind::BooleanLiteral(value) => {
            Conform::new(builtin_types.bool(), UnaryCast::SpecializeBoolean(*value))
        }
        TypeKind::IntegerLiteralInRange(min, max) => default_integer_types
            .into_iter()
            .flat_map(|possible_type| {
                default_from_integer_literal(
                    ctx,
                    max,
                    assumptions,
                    source,
                    &possible_type.kind,
                    builtin_types,
                    target,
                )
                .and_then(|_| {
                    default_from_integer_literal(
                        ctx,
                        min,
                        assumptions,
                        source,
                        &possible_type.kind,
                        builtin_types,
                        target,
                    )
                })
            })
            .map(|found| {
                Conform::new(
                    found.ty,
                    UnaryCast::Extend(found.ty.0.kind.bit_integer_sign().unwrap()),
                )
            })
            .next()
            .ok_or_else(|| {
                ErrorDiagnostic::new(
                    "Possible integers are too large to represent without concrete type",
                    source,
                )
            })?,
        TypeKind::NullLiteral => Conform::new(
            builtin_types.ptr_void(),
            UnaryCast::SpecializePointerOuter(builtin_types.ptr_void()),
        ),
        TypeKind::AsciiCharLiteral(value) => {
            Conform::new(builtin_types.u8(), UnaryCast::SpecializeAsciiChar(*value))
        }
        _ => Conform::identity(ty),
    };

    Ok(inner_conform.after_implicit_dereferences(ctx, original_from_ty, dereferences))
}

fn default_from_integer_literal<'env>(
    ctx: &mut ExecutionCtx<'env>,
    value: &'env BigInt,
    assumptions: CIntegerAssumptions,
    source: Source,
    to_type_kind: &'env TypeKind<'env>,
    builtin_types: &'env BuiltinTypes<'env>,
    target: &Target,
) -> Option<Conform<'env>> {
    match &to_type_kind {
        TypeKind::Floating(float_size) => value.to_f64().map(|float| {
            Conform::new(
                builtin_types.floating(*float_size),
                UnaryCast::SpecializeFloat(NotNan::new(float).ok()),
            )
        }),
        TypeKind::BitInteger(to_bits, to_sign) => {
            does_integer_literal_fit(value, *to_bits, *to_sign).then(|| {
                Conform::new(
                    UnaliasedType(ctx.alloc(TypeKind::BitInteger(*to_bits, *to_sign).at(source))),
                    UnaryCast::SpecializeInteger(value).into(),
                )
            })
        }
        TypeKind::CInteger(to_c_integer, to_sign) => {
            does_integer_literal_fit_in_c(value, *to_c_integer, *to_sign, assumptions, target).then(
                || {
                    Conform::new(
                        UnaliasedType(
                            ctx.alloc(TypeKind::CInteger(*to_c_integer, *to_sign).at(source)),
                        ),
                        UnaryCast::SpecializeInteger(value),
                    )
                },
            )
        }
        TypeKind::SizeInteger(to_sign) => {
            // Size types (i.e. size_t, ssize_t, usize, isize) are guananteed to be at least 16 bits
            // Anything more than that will require explicit casts
            let does_fit = match to_sign {
                IntegerSign::Signed => i16::try_from(value).is_ok(),
                IntegerSign::Unsigned => u16::try_from(value).is_ok(),
            };

            does_fit.then(|| {
                Conform::new(
                    UnaliasedType(ctx.alloc(TypeKind::SizeInteger(*to_sign).at(source))),
                    UnaryCast::SpecializeInteger(value).into(),
                )
            })
        }
        _ => None,
    }
}
