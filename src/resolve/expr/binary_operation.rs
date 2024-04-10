use super::{resolve_expr, ResolveExprCtx};
use crate::{
    ast::{self, Source},
    resolve::{
        error::{ResolveError, ResolveErrorKind},
        unify_types::unify_types, Initialized,
    },
    resolved::{self, FloatOrInteger, FloatOrSign, NumericMode, TypedExpr},
};
use ast::{IntegerBits, IntegerSign};

pub fn resolve_binary_operation_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    binary_operation: &ast::BinaryOperation,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let mut left = resolve_expr(ctx, &binary_operation.left, Initialized::Require)?;
    let mut right = resolve_expr(ctx, &binary_operation.right, Initialized::Require)?;

    let unified_type = unify_types(&mut [&mut left, &mut right]).ok_or_else(|| {
        ResolveError::new(
            ctx.resolved_ast.source_file_cache,
            source,
            ResolveErrorKind::IncompatibleTypesForBinaryOperator {
                operator: binary_operation.operator.to_string(),
                left: left.resolved_type.to_string(),
                right: right.resolved_type.to_string(),
            },
        )
    })?;

    let operator =
        match binary_operation.operator {
            ast::BinaryOperator::Add => {
                numeric_mode_from_type(&unified_type).map(resolved::BinaryOperator::Add)
            }
            ast::BinaryOperator::Subtract => {
                numeric_mode_from_type(&unified_type).map(resolved::BinaryOperator::Subtract)
            }
            ast::BinaryOperator::Multiply => {
                numeric_mode_from_type(&unified_type).map(resolved::BinaryOperator::Multiply)
            }
            ast::BinaryOperator::Divide => {
                float_or_sign_from_type(&unified_type, false).map(resolved::BinaryOperator::Divide)
            }
            ast::BinaryOperator::Modulus => {
                float_or_sign_from_type(&unified_type, false).map(resolved::BinaryOperator::Modulus)
            }
            ast::BinaryOperator::Equals => float_or_integer_from_type(&unified_type, true)
                .map(resolved::BinaryOperator::Equals),
            ast::BinaryOperator::NotEquals => float_or_integer_from_type(&unified_type, true)
                .map(resolved::BinaryOperator::NotEquals),
            ast::BinaryOperator::LessThan => float_or_sign_from_type(&unified_type, false)
                .map(resolved::BinaryOperator::LessThan),
            ast::BinaryOperator::LessThanEq => float_or_sign_from_type(&unified_type, false)
                .map(resolved::BinaryOperator::LessThanEq),
            ast::BinaryOperator::GreaterThan => float_or_sign_from_type(&unified_type, false)
                .map(resolved::BinaryOperator::GreaterThan),
            ast::BinaryOperator::GreaterThanEq => float_or_sign_from_type(&unified_type, false)
                .map(resolved::BinaryOperator::GreaterThanEq),
            ast::BinaryOperator::BitwiseAnd => matches!(
                unified_type,
                resolved::Type::Integer { .. } | resolved::Type::Boolean
            )
            .then_some(resolved::BinaryOperator::BitwiseAnd),
            ast::BinaryOperator::BitwiseOr => matches!(
                unified_type,
                resolved::Type::Integer { .. } | resolved::Type::Boolean
            )
            .then_some(resolved::BinaryOperator::BitwiseOr),
            ast::BinaryOperator::BitwiseXor => matches!(
                unified_type,
                resolved::Type::Integer { .. } | resolved::Type::Boolean
            )
            .then_some(resolved::BinaryOperator::BitwiseXor),
            ast::BinaryOperator::LeftShift => match unified_type {
                resolved::Type::Integer { sign, .. } => Some(match sign {
                    IntegerSign::Signed => resolved::BinaryOperator::LeftShift,
                    IntegerSign::Unsigned => resolved::BinaryOperator::LogicalLeftShift,
                }),
                _ => None,
            },
            ast::BinaryOperator::RightShift => match unified_type {
                resolved::Type::Integer { sign, .. } => Some(match sign {
                    IntegerSign::Signed => resolved::BinaryOperator::RightShift,
                    IntegerSign::Unsigned => resolved::BinaryOperator::LogicalRightShift,
                }),
                _ => None,
            },
            ast::BinaryOperator::LogicalLeftShift => {
                matches!(unified_type, resolved::Type::Integer { .. })
                    .then_some(resolved::BinaryOperator::BitwiseXor)
            }
            ast::BinaryOperator::LogicalRightShift => {
                matches!(unified_type, resolved::Type::Integer { .. })
                    .then_some(resolved::BinaryOperator::BitwiseXor)
            }
        };

    let operator = operator.ok_or_else(|| {
        ResolveError::new(
            ctx.resolved_ast.source_file_cache,
            source,
            ResolveErrorKind::CannotPerformBinaryOperationForType {
                operator: binary_operation.operator.to_string(),
                bad_type: unified_type.to_string(),
            },
        )
    })?;

    let result_type = binary_operation
        .operator
        .returns_boolean()
        .then_some(resolved::Type::Boolean)
        .unwrap_or(unified_type);

    Ok(TypedExpr::new(
        result_type,
        resolved::Expr::new(
            resolved::ExprKind::BinaryOperation(Box::new(resolved::BinaryOperation {
                operator,
                left,
                right,
            })),
            source,
        ),
    ))
}

fn float_or_integer_from_type(
    unified_type: &resolved::Type,
    allow_on_bools: bool,
) -> Option<FloatOrInteger> {
    match unified_type {
        resolved::Type::Boolean if allow_on_bools => Some(FloatOrInteger::Integer),
        resolved::Type::Integer { .. } => Some(FloatOrInteger::Integer),
        resolved::Type::Float(_) => Some(FloatOrInteger::Float),
        _ => None,
    }
}

fn float_or_sign_from_type(
    unified_type: &resolved::Type,
    allow_on_bools: bool,
) -> Option<FloatOrSign> {
    match unified_type {
        resolved::Type::Boolean if allow_on_bools => {
            Some(FloatOrSign::Integer(IntegerSign::Unsigned))
        }
        resolved::Type::Integer { sign, .. } => Some(FloatOrSign::Integer(*sign)),
        resolved::Type::Float(_) => Some(FloatOrSign::Float),
        _ => None,
    }
}

fn numeric_mode_from_type(unified_type: &resolved::Type) -> Option<NumericMode> {
    match unified_type {
        resolved::Type::Integer { sign, bits } => Some(match bits {
            IntegerBits::Normal => NumericMode::CheckOverflow(*sign),
            _ => NumericMode::Integer(*sign),
        }),
        resolved::Type::Float(_) => Some(NumericMode::Float),
        _ => None,
    }
}
