use super::{from_integer_literal::from_integer_literal, Objective, ObjectiveResult, Perform};
use crate::{
    ast::{CIntegerAssumptions, FloatSize},
    resolve::error::{ResolveError, ResolveErrorKind},
    resolved::{Expr, ExprKind, TypeKind, TypedExpr},
    source_files::Source,
};
use num::BigInt;

pub fn conform_expr_to_default(
    mut expr: TypedExpr,
    c_integer_assumptions: CIntegerAssumptions,
) -> Result<TypedExpr, ResolveError> {
    let source = expr.expr.source;

    match conform_expr_to_default_via_mut::<Perform>(&mut expr, c_integer_assumptions) {
        Ok(()) => Ok(expr),
        Err(()) => Err(ResolveErrorKind::Other {
            message: "Failed to conform to default".into(),
        }
        .at(source)),
    }
}

pub fn conform_expr_to_default_via_mut<O: Objective>(
    expr: &mut TypedExpr,
    c_integer_assumptions: CIntegerAssumptions,
) -> Result<(), ()> {
    match &expr.resolved_type.kind {
        TypeKind::IntegerLiteral(value) => {
            *expr = conform_integer_literal_to_default::<Perform>(
                value,
                c_integer_assumptions,
                expr.expr.source,
            )?;
        }
        TypeKind::FloatLiteral(value) => {
            *expr = conform_float_literal_to_default(*value, expr.expr.source);
        }
        _ => (),
    }

    Ok(())
}

pub fn conform_float_literal_to_default(value: f64, source: Source) -> TypedExpr {
    TypedExpr::new(
        TypeKind::f64().at(source),
        Expr::new(ExprKind::FloatingLiteral(FloatSize::Bits64, value), source),
    )
}

pub fn conform_integer_literal_to_default<O: Objective>(
    value: &BigInt,
    c_integer_assumptions: CIntegerAssumptions,
    source: Source,
) -> ObjectiveResult<O> {
    if let Some(found) = [
        TypeKind::i32().at(source),
        TypeKind::u32().at(source),
        TypeKind::i64().at(source),
        TypeKind::u64().at(source),
    ]
    .iter()
    .flat_map(|possible_type| {
        from_integer_literal::<O>(value, c_integer_assumptions, source, possible_type)
    })
    .next()
    {
        return Ok(found);
    }

    O::fail()
}
