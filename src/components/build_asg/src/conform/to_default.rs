use super::{Objective, ObjectiveResult, Perform, from_integer_literal::from_integer_literal};
use crate::error::{ResolveError, ResolveErrorKind};
use asg::{Expr, ExprKind, TypeKind, TypedExpr};
use num::BigInt;
use ordered_float::NotNan;
use primitives::{CIntegerAssumptions, FloatSize};
use source_files::Source;

pub fn conform_expr_to_default_or_error(
    expr: impl TypedExprLike,
    c_integer_assumptions: CIntegerAssumptions,
) -> Result<TypedExpr, ResolveError> {
    let source = expr.view().expr.source;

    conform_expr_to_default::<Perform>(expr, c_integer_assumptions).map_err(|()| {
        ResolveErrorKind::Other {
            message: "Failed to conform".into(),
        }
        .at(source)
    })
}

pub fn conform_expr_to_default<O: Objective>(
    expr: impl TypedExprLike,
    c_integer_assumptions: CIntegerAssumptions,
) -> ObjectiveResult<O> {
    let source = expr.view().expr.source;

    match &expr.view().ty.kind {
        TypeKind::IntegerLiteral(value) => {
            conform_integer_literal_to_default::<O>(value, c_integer_assumptions, source)
        }
        TypeKind::FloatLiteral(value) => conform_float_literal_to_default::<O>(*value, source),
        _ => O::success(|| expr.make()),
    }
}

pub fn conform_float_literal_to_default<O: Objective>(
    value: Option<NotNan<f64>>,
    source: Source,
) -> ObjectiveResult<O> {
    O::success(|| {
        TypedExpr::new(
            TypeKind::f64().at(source),
            Expr::new(ExprKind::FloatingLiteral(FloatSize::Bits64, value), source),
        )
    })
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

pub trait TypedExprLike {
    fn make(self) -> TypedExpr;
    fn view(&self) -> &TypedExpr;
}

impl TypedExprLike for TypedExpr {
    fn make(self) -> TypedExpr {
        self
    }

    fn view(&self) -> &TypedExpr {
        self
    }
}

impl TypedExprLike for &TypedExpr {
    fn make(self) -> TypedExpr {
        self.clone()
    }

    fn view(&self) -> &TypedExpr {
        self
    }
}

impl TypedExprLike for &mut TypedExpr {
    fn make(self) -> TypedExpr {
        self.clone()
    }

    fn view(&self) -> &TypedExpr {
        *self
    }
}
