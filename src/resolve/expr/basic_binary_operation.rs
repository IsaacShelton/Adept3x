use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast::{self, ConformBehavior},
    resolve::{
        error::{ResolveError, ResolveErrorKind},
        unify_types::unify_types,
        Initialized,
    },
    resolved::{self, FloatOrInteger, FloatOrSignLax, NumericMode, SignOrIndeterminate, TypedExpr},
    source_files::Source,
};
use ast::IntegerSign;

pub fn resolve_basic_binary_operation_expr(
    ctx: &mut ResolveExprCtx<'_, '_>,
    binary_operation: &ast::BasicBinaryOperation,
    preferred_type: Option<PreferredType>,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let mut left = resolve_expr(
        ctx,
        &binary_operation.left,
        preferred_type,
        Initialized::Require,
    )?;

    let mut right = resolve_expr(
        ctx,
        &binary_operation.right,
        preferred_type,
        Initialized::Require,
    )?;

    let unified_type = unify_types(
        preferred_type.map(|preferred_type| preferred_type.view(ctx.resolved_ast)),
        &mut [&mut left, &mut right],
        ConformBehavior::Adept,
        source,
    )
    .ok_or_else(|| {
        ResolveErrorKind::IncompatibleTypesForBinaryOperator {
            operator: binary_operation.operator.to_string(),
            left: left.resolved_type.to_string(),
            right: right.resolved_type.to_string(),
        }
        .at(source)
    })?;

    let operator =
        resolve_basic_binary_operator(&binary_operation.operator, &unified_type, source)?;

    let result_type = if binary_operation.operator.returns_boolean() {
        resolved::TypeKind::Boolean.at(source)
    } else {
        unified_type
    };

    Ok(TypedExpr::new(
        result_type,
        resolved::Expr::new(
            resolved::ExprKind::BasicBinaryOperation(Box::new(resolved::BasicBinaryOperation {
                operator,
                left,
                right,
            })),
            source,
        ),
    ))
}

pub fn resolve_basic_binary_operator(
    ast_operator: &ast::BasicBinaryOperator,
    resolved_type: &resolved::Type,
    source: Source,
) -> Result<resolved::BasicBinaryOperator, ResolveError> {
    let resolved_operator = match ast_operator {
        ast::BasicBinaryOperator::Add => {
            numeric_mode_from_type(resolved_type).map(resolved::BasicBinaryOperator::Add)
        }
        ast::BasicBinaryOperator::Subtract => {
            numeric_mode_from_type(resolved_type).map(resolved::BasicBinaryOperator::Subtract)
        }
        ast::BasicBinaryOperator::Multiply => {
            numeric_mode_from_type(resolved_type).map(resolved::BasicBinaryOperator::Multiply)
        }
        ast::BasicBinaryOperator::Divide => float_or_sign_lax_from_type(resolved_type, false)
            .map(resolved::BasicBinaryOperator::Divide),
        ast::BasicBinaryOperator::Modulus => float_or_sign_lax_from_type(resolved_type, false)
            .map(resolved::BasicBinaryOperator::Modulus),
        ast::BasicBinaryOperator::Equals => float_or_integer_from_type(resolved_type, true)
            .map(resolved::BasicBinaryOperator::Equals),
        ast::BasicBinaryOperator::NotEquals => float_or_integer_from_type(resolved_type, true)
            .map(resolved::BasicBinaryOperator::NotEquals),
        ast::BasicBinaryOperator::LessThan => float_or_sign_lax_from_type(resolved_type, false)
            .map(resolved::BasicBinaryOperator::LessThan),
        ast::BasicBinaryOperator::LessThanEq => float_or_sign_lax_from_type(resolved_type, false)
            .map(resolved::BasicBinaryOperator::LessThanEq),
        ast::BasicBinaryOperator::GreaterThan => float_or_sign_lax_from_type(resolved_type, false)
            .map(resolved::BasicBinaryOperator::GreaterThan),
        ast::BasicBinaryOperator::GreaterThanEq => {
            float_or_sign_lax_from_type(resolved_type, false)
                .map(resolved::BasicBinaryOperator::GreaterThanEq)
        }
        ast::BasicBinaryOperator::BitwiseAnd => (resolved_type.kind.is_integer()
            || resolved_type.kind.is_c_integer()
            || resolved_type.kind.is_boolean())
        .then_some(resolved::BasicBinaryOperator::BitwiseAnd),
        ast::BasicBinaryOperator::BitwiseOr => (resolved_type.kind.is_integer()
            || resolved_type.kind.is_c_integer()
            || resolved_type.kind.is_boolean())
        .then_some(resolved::BasicBinaryOperator::BitwiseOr),
        ast::BasicBinaryOperator::BitwiseXor => (resolved_type.kind.is_integer()
            || resolved_type.kind.is_c_integer())
        .then_some(resolved::BasicBinaryOperator::BitwiseXor),
        ast::BasicBinaryOperator::LeftShift | ast::BasicBinaryOperator::LogicalLeftShift => {
            (resolved_type.kind.is_integer() || resolved_type.kind.is_c_integer())
                .then_some(resolved::BasicBinaryOperator::LogicalLeftShift)
        }
        ast::BasicBinaryOperator::RightShift => match resolved_type.kind {
            resolved::TypeKind::Integer(_, sign) => {
                Some(resolved::BasicBinaryOperator::ArithmeticRightShift(
                    SignOrIndeterminate::Sign(sign),
                ))
            }
            resolved::TypeKind::CInteger(c_integer, sign) => Some(if let Some(sign) = sign {
                resolved::BasicBinaryOperator::ArithmeticRightShift(SignOrIndeterminate::Sign(sign))
            } else {
                resolved::BasicBinaryOperator::ArithmeticRightShift(
                    SignOrIndeterminate::Indeterminate(c_integer),
                )
            }),
            _ => None,
        },
        ast::BasicBinaryOperator::LogicalRightShift => (resolved_type.kind.is_integer()
            || resolved_type.kind.is_c_integer())
        .then_some(resolved::BasicBinaryOperator::LogicalRightShift),
    };

    resolved_operator.ok_or_else(|| {
        ResolveErrorKind::CannotPerformBinaryOperationForType {
            operator: ast_operator.to_string(),
            bad_type: resolved_type.to_string(),
        }
        .at(source)
    })
}

fn float_or_integer_from_type(
    unified_type: &resolved::Type,
    allow_on_bools: bool,
) -> Option<FloatOrInteger> {
    match &unified_type.kind {
        resolved::TypeKind::Boolean if allow_on_bools => Some(FloatOrInteger::Integer),
        resolved::TypeKind::Integer(..) | resolved::TypeKind::CInteger(..) => {
            Some(FloatOrInteger::Integer)
        }
        resolved::TypeKind::Floating(_) => Some(FloatOrInteger::Float),
        _ => None,
    }
}

fn float_or_sign_lax_from_type(
    unified_type: &resolved::Type,
    allow_on_bools: bool,
) -> Option<FloatOrSignLax> {
    match &unified_type.kind {
        resolved::TypeKind::Boolean if allow_on_bools => {
            Some(FloatOrSignLax::Integer(IntegerSign::Unsigned))
        }
        resolved::TypeKind::Integer(_, sign) => Some(FloatOrSignLax::Integer(*sign)),
        resolved::TypeKind::CInteger(c_integer, sign) => {
            if let Some(sign) = sign {
                Some(FloatOrSignLax::Integer(*sign))
            } else {
                Some(FloatOrSignLax::IndeterminateInteger(*c_integer))
            }
        }
        resolved::TypeKind::Floating(_) => Some(FloatOrSignLax::Float),
        _ => None,
    }
}

fn numeric_mode_from_type(unified_type: &resolved::Type) -> Option<NumericMode> {
    match &unified_type.kind {
        resolved::TypeKind::Integer(_, sign) => Some(NumericMode::Integer(*sign)),
        resolved::TypeKind::CInteger(c_integer, sign) => {
            if let Some(sign) = sign {
                Some(NumericMode::Integer(*sign))
            } else {
                Some(NumericMode::LooseIndeterminateSignInteger(*c_integer))
            }
        }
        resolved::TypeKind::Floating(_) => Some(NumericMode::Float),
        _ => None,
    }
}
