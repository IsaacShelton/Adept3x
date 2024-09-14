use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, UnaryMathOperator},
    resolve::{
        conform::to_default::conform_integer_literal_to_default_or_error,
        error::{ResolveError, ResolveErrorKind},
        Initialized,
    },
    resolved::{Expr, ExprKind, TypeKind, TypedExpr, UnaryMathOperation},
    source_files::Source,
};

pub fn resolve_unary_math_operation_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    operator: &UnaryMathOperator,
    inner: &ast::Expr,
    preferred_type: Option<PreferredType>,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let resolved_expr = resolve_expr(ctx, inner, preferred_type, Initialized::Require)?;

    if operator.is_dereference() {
        if resolved_expr.resolved_type.kind.is_ambiguous_type() {
            return Err(ResolveErrorKind::CannotPerformUnaryOperationForType {
                operator: operator.to_string(),
                bad_type: resolved_expr.resolved_type.to_string(),
            }
            .at(source));
        }

        let result_type = if let TypeKind::Pointer(inner) = &resolved_expr.resolved_type.kind {
            (**inner).clone()
        } else {
            return Err(ResolveErrorKind::CannotPerformUnaryOperationForType {
                operator: operator.to_string(),
                bad_type: resolved_expr.resolved_type.to_string(),
            }
            .at(source));
        };

        let expr = Expr::new(
            ExprKind::UnaryMathOperation(Box::new(UnaryMathOperation {
                operator: operator.clone(),
                inner: resolved_expr,
            })),
            source,
        );

        return Ok(TypedExpr::new(result_type, expr));
    }

    let resolved_expr = match &resolved_expr.resolved_type.kind {
        TypeKind::Boolean => resolved_expr,
        TypeKind::Integer(..) => resolved_expr,
        TypeKind::IntegerLiteral(value) => {
            conform_integer_literal_to_default_or_error(value, resolved_expr.expr.source)?
        }
        _ => {
            return Err(ResolveErrorKind::CannotPerformUnaryOperationForType {
                operator: operator.to_string(),
                bad_type: resolved_expr.resolved_type.to_string(),
            }
            .at(source));
        }
    };

    let result_type = match operator {
        UnaryMathOperator::Not | UnaryMathOperator::IsNonZero => TypeKind::Boolean.at(source),
        UnaryMathOperator::BitComplement | UnaryMathOperator::Negate => {
            resolved_expr.resolved_type.clone()
        }
        UnaryMathOperator::Dereference => {
            unreachable!("should've already handled address-of/dereference operators")
        }
    };

    let expr = Expr::new(
        ExprKind::UnaryMathOperation(Box::new(UnaryMathOperation {
            operator: operator.clone(),
            inner: resolved_expr,
        })),
        source,
    );

    Ok(TypedExpr::new(result_type, expr))
}
