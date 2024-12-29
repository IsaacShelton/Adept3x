use super::{resolve_expr, PreferredType, ResolveExprCtx};
use crate::{
    ast,
    resolve::{
        error::{ResolveError, ResolveErrorKind},
        unify_types::unify_types,
        Initialized,
    },
    asg::{
        self, Constraint, FloatOrInteger, FloatOrSignLax, NumericMode, SignOrIndeterminate,
        TypedExpr,
    },
    source_files::Source,
};
use ast::IntegerSign;
use num::BigInt;
use num_bigint::Sign;

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

    if let asg::TypeKind::IntegerLiteral(left) = &left.ty.kind {
        if let asg::TypeKind::IntegerLiteral(right) = &right.ty.kind {
            return resolve_basic_binary_operation_expr_on_literals(
                &binary_operation.operator,
                left,
                right,
                source,
            );
        }
    }

    let unified_type = unify_types(
        ctx,
        preferred_type.map(|preferred_type| preferred_type.view(ctx.asg)),
        &mut [&mut left, &mut right],
        ctx.adept_conform_behavior(),
        source,
    )
    .ok_or_else(|| {
        ResolveErrorKind::IncompatibleTypesForBinaryOperator {
            operator: binary_operation.operator.to_string(),
            left: left.ty.to_string(),
            right: right.ty.to_string(),
        }
        .at(source)
    })?;

    let operator =
        resolve_basic_binary_operator(ctx, &binary_operation.operator, &unified_type, source)?;

    let result_type = if binary_operation.operator.returns_boolean() {
        asg::TypeKind::Boolean.at(source)
    } else {
        unified_type
    };

    Ok(TypedExpr::new(
        result_type,
        asg::Expr::new(
            asg::ExprKind::BasicBinaryOperation(Box::new(asg::BasicBinaryOperation {
                operator,
                left,
                right,
            })),
            source,
        ),
    ))
}

pub fn resolve_basic_binary_operator(
    ctx: &ResolveExprCtx,
    ast_operator: &ast::BasicBinaryOperator,
    ty: &asg::Type,
    source: Source,
) -> Result<asg::BasicBinaryOperator, ResolveError> {
    let resolved_operator = match ast_operator {
        ast::BasicBinaryOperator::Add => NumericMode::try_new(ty)
            .map(asg::BasicBinaryOperator::Add)
            .or_else(|| {
                ctx.current_constraints
                    .satisfies(ty, &Constraint::PrimitiveAdd)
                    .then(|| asg::BasicBinaryOperator::PrimitiveAdd(ty.clone()))
            }),
        ast::BasicBinaryOperator::Subtract => {
            NumericMode::try_new(ty).map(asg::BasicBinaryOperator::Subtract)
        }
        ast::BasicBinaryOperator::Multiply => {
            NumericMode::try_new(ty).map(asg::BasicBinaryOperator::Multiply)
        }
        ast::BasicBinaryOperator::Divide => float_or_sign_lax_from_type(ty, false)
            .map(asg::BasicBinaryOperator::Divide),
        ast::BasicBinaryOperator::Modulus => float_or_sign_lax_from_type(ty, false)
            .map(asg::BasicBinaryOperator::Modulus),
        ast::BasicBinaryOperator::Equals => float_or_integer_from_type(ty, true)
            .map(asg::BasicBinaryOperator::Equals),
        ast::BasicBinaryOperator::NotEquals => float_or_integer_from_type(ty, true)
            .map(asg::BasicBinaryOperator::NotEquals),
        ast::BasicBinaryOperator::LessThan => float_or_sign_lax_from_type(ty, false)
            .map(asg::BasicBinaryOperator::LessThan),
        ast::BasicBinaryOperator::LessThanEq => float_or_sign_lax_from_type(ty, false)
            .map(asg::BasicBinaryOperator::LessThanEq),
        ast::BasicBinaryOperator::GreaterThan => float_or_sign_lax_from_type(ty, false)
            .map(asg::BasicBinaryOperator::GreaterThan),
        ast::BasicBinaryOperator::GreaterThanEq => {
            float_or_sign_lax_from_type(ty, false)
                .map(asg::BasicBinaryOperator::GreaterThanEq)
        }
        ast::BasicBinaryOperator::BitwiseAnd => (ty.kind.is_integer()
            || ty.kind.is_c_integer()
            || ty.kind.is_boolean())
        .then_some(asg::BasicBinaryOperator::BitwiseAnd),
        ast::BasicBinaryOperator::BitwiseOr => (ty.kind.is_integer()
            || ty.kind.is_c_integer()
            || ty.kind.is_boolean())
        .then_some(asg::BasicBinaryOperator::BitwiseOr),
        ast::BasicBinaryOperator::BitwiseXor => (ty.kind.is_integer()
            || ty.kind.is_c_integer())
        .then_some(asg::BasicBinaryOperator::BitwiseXor),
        ast::BasicBinaryOperator::LeftShift | ast::BasicBinaryOperator::LogicalLeftShift => {
            (ty.kind.is_integer() || ty.kind.is_c_integer())
                .then_some(asg::BasicBinaryOperator::LogicalLeftShift)
        }
        ast::BasicBinaryOperator::RightShift => match ty.kind {
            asg::TypeKind::Integer(_, sign) => {
                Some(asg::BasicBinaryOperator::ArithmeticRightShift(
                    SignOrIndeterminate::Sign(sign),
                ))
            }
            asg::TypeKind::CInteger(c_integer, sign) => Some(if let Some(sign) = sign {
                asg::BasicBinaryOperator::ArithmeticRightShift(SignOrIndeterminate::Sign(sign))
            } else {
                asg::BasicBinaryOperator::ArithmeticRightShift(
                    SignOrIndeterminate::Indeterminate(c_integer),
                )
            }),
            _ => None,
        },
        ast::BasicBinaryOperator::LogicalRightShift => (ty.kind.is_integer()
            || ty.kind.is_c_integer())
        .then_some(asg::BasicBinaryOperator::LogicalRightShift),
    };

    resolved_operator.ok_or_else(|| {
        ResolveErrorKind::CannotPerformBinaryOperationForType {
            operator: ast_operator.to_string(),
            bad_type: ty.to_string(),
        }
        .at(source)
    })
}

fn float_or_integer_from_type(
    unified_type: &asg::Type,
    allow_on_bools: bool,
) -> Option<FloatOrInteger> {
    match &unified_type.kind {
        asg::TypeKind::Boolean if allow_on_bools => Some(FloatOrInteger::Integer),
        asg::TypeKind::Integer(..) | asg::TypeKind::CInteger(..) => {
            Some(FloatOrInteger::Integer)
        }
        asg::TypeKind::Floating(_) => Some(FloatOrInteger::Float),
        _ => None,
    }
}

fn float_or_sign_lax_from_type(
    unified_type: &asg::Type,
    allow_on_bools: bool,
) -> Option<FloatOrSignLax> {
    match &unified_type.kind {
        asg::TypeKind::Boolean if allow_on_bools => {
            Some(FloatOrSignLax::Integer(IntegerSign::Unsigned))
        }
        asg::TypeKind::Integer(_, sign) => Some(FloatOrSignLax::Integer(*sign)),
        asg::TypeKind::CInteger(c_integer, sign) => {
            if let Some(sign) = sign {
                Some(FloatOrSignLax::Integer(*sign))
            } else {
                Some(FloatOrSignLax::IndeterminateInteger(*c_integer))
            }
        }
        asg::TypeKind::Floating(_) => Some(FloatOrSignLax::Float),
        _ => None,
    }
}

pub fn resolve_basic_binary_operation_expr_on_literals(
    operator: &ast::BasicBinaryOperator,
    left: &BigInt,
    right: &BigInt,
    source: Source,
) -> Result<TypedExpr, ResolveError> {
    let result = match operator {
        ast::BasicBinaryOperator::Add => left + right,
        ast::BasicBinaryOperator::Subtract => left - right,
        ast::BasicBinaryOperator::Multiply => left * right,
        ast::BasicBinaryOperator::Divide => left
            .checked_div(right)
            .ok_or_else(|| ResolveErrorKind::DivideByZero.at(source))?,
        ast::BasicBinaryOperator::Modulus => {
            if *right == BigInt::ZERO {
                return Err(ResolveErrorKind::DivideByZero.at(source));
            } else {
                left % right
            }
        }
        ast::BasicBinaryOperator::Equals => {
            return Ok(TypedExpr::new(
                asg::TypeKind::Boolean.at(source),
                asg::ExprKind::BooleanLiteral(left == right).at(source),
            ))
        }
        ast::BasicBinaryOperator::NotEquals => {
            return Ok(TypedExpr::new(
                asg::TypeKind::Boolean.at(source),
                asg::ExprKind::BooleanLiteral(left != right).at(source),
            ))
        }
        ast::BasicBinaryOperator::LessThan => {
            return Ok(TypedExpr::new(
                asg::TypeKind::Boolean.at(source),
                asg::ExprKind::BooleanLiteral(left < right).at(source),
            ))
        }
        ast::BasicBinaryOperator::LessThanEq => {
            return Ok(TypedExpr::new(
                asg::TypeKind::Boolean.at(source),
                asg::ExprKind::BooleanLiteral(left <= right).at(source),
            ))
        }
        ast::BasicBinaryOperator::GreaterThan => {
            return Ok(TypedExpr::new(
                asg::TypeKind::Boolean.at(source),
                asg::ExprKind::BooleanLiteral(left >= right).at(source),
            ))
        }
        ast::BasicBinaryOperator::GreaterThanEq => {
            return Ok(TypedExpr::new(
                asg::TypeKind::Boolean.at(source),
                asg::ExprKind::BooleanLiteral(left > right).at(source),
            ))
        }
        ast::BasicBinaryOperator::BitwiseAnd => {
            return Err(ResolveErrorKind::CannotPerformOnUnspecializedInteger {
                operation: "bitwise-and".into(),
            }
            .at(source))
        }
        ast::BasicBinaryOperator::BitwiseOr => {
            return Err(ResolveErrorKind::CannotPerformOnUnspecializedInteger {
                operation: "bitwise-or".into(),
            }
            .at(source))
        }
        ast::BasicBinaryOperator::BitwiseXor => {
            return Err(ResolveErrorKind::CannotPerformOnUnspecializedInteger {
                operation: "bitwise-xor".into(),
            }
            .at(source))
        }
        ast::BasicBinaryOperator::LeftShift | ast::BasicBinaryOperator::LogicalLeftShift => {
            if left.sign() == Sign::Minus {
                return Err(ResolveErrorKind::ShiftByNegative.at(source));
            } else if let Ok(small) = u64::try_from(right) {
                left.clone() << small
            } else {
                return Err(ResolveErrorKind::ShiftByNegative.at(source));
            }
        }
        ast::BasicBinaryOperator::RightShift => {
            if left.sign() == Sign::Minus {
                return Err(ResolveErrorKind::ShiftByNegative.at(source));
            } else if let Ok(small) = u64::try_from(right) {
                left.clone() >> small
            } else {
                return Err(ResolveErrorKind::ShiftByNegative.at(source));
            }
        }
        ast::BasicBinaryOperator::LogicalRightShift => {
            return Err(ResolveErrorKind::CannotPerformOnUnspecializedInteger {
                operation: "perform logical right shift on".into(),
            }
            .at(source))
        }
    };

    return Ok(TypedExpr::new(
        asg::TypeKind::IntegerLiteral(result.clone()).at(source),
        asg::ExprKind::IntegerLiteral(result).at(source),
    ));
}
