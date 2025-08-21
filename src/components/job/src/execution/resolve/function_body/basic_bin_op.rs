use crate::{
    BasicBinaryOperator, ExecutionCtx, Resolved,
    repr::{TypeKind, UnaliasedType},
};
use diagnostics::ErrorDiagnostic;
use num_bigint::{BigInt, Sign};
use primitives::{FloatOrInteger, FloatOrSignLax, IntegerSign, SignOrIndeterminate};
use source_files::Source;

pub fn resolve_basic_binary_operation_expr_on_literals<'env>(
    ctx: &mut ExecutionCtx<'env>,
    operator: &ast::BasicBinaryOperator,
    left: &BigInt,
    right: &BigInt,
    source: Source,
) -> Result<Resolved<'env>, ErrorDiagnostic> {
    let result = match operator {
        ast::BasicBinaryOperator::Add => left + right,
        ast::BasicBinaryOperator::Subtract => left - right,
        ast::BasicBinaryOperator::Multiply => left * right,
        ast::BasicBinaryOperator::Divide => left
            .checked_div(right)
            .ok_or_else(|| ErrorDiagnostic::new("Cannot divide by zero", source))?,
        ast::BasicBinaryOperator::Modulus => {
            if *right == BigInt::ZERO {
                return Err(ErrorDiagnostic::new("Cannot modulo by zero", source));
            } else {
                left % right
            }
        }
        ast::BasicBinaryOperator::Equals => {
            return Ok(Resolved::from_type(UnaliasedType(
                ctx.alloc(TypeKind::BooleanLiteral(left == right).at(source)),
            )));
        }
        ast::BasicBinaryOperator::NotEquals => {
            return Ok(Resolved::from_type(UnaliasedType(
                ctx.alloc(TypeKind::BooleanLiteral(left != right).at(source)),
            )));
        }
        ast::BasicBinaryOperator::LessThan => {
            return Ok(Resolved::from_type(UnaliasedType(
                ctx.alloc(TypeKind::BooleanLiteral(left < right).at(source)),
            )));
        }
        ast::BasicBinaryOperator::LessThanEq => {
            return Ok(Resolved::from_type(UnaliasedType(
                ctx.alloc(TypeKind::BooleanLiteral(left < right).at(source)),
            )));
        }
        ast::BasicBinaryOperator::GreaterThan => {
            return Ok(Resolved::from_type(UnaliasedType(
                ctx.alloc(TypeKind::BooleanLiteral(left > right).at(source)),
            )));
        }
        ast::BasicBinaryOperator::GreaterThanEq => {
            return Ok(Resolved::from_type(UnaliasedType(
                ctx.alloc(TypeKind::BooleanLiteral(left >= right).at(source)),
            )));
        }
        ast::BasicBinaryOperator::BitwiseAnd => {
            return Err(ErrorDiagnostic::new(
                "Cannot perform bitwise-and on unspecialized integer",
                source,
            ));
        }
        ast::BasicBinaryOperator::BitwiseOr => {
            return Err(ErrorDiagnostic::new(
                "Cannot perform bitwise-or on unspecialized integer",
                source,
            ));
        }
        ast::BasicBinaryOperator::BitwiseXor => {
            return Err(ErrorDiagnostic::new(
                "Cannot perform bitwise-xor on unspecialized integer",
                source,
            ));
        }
        ast::BasicBinaryOperator::LeftShift | ast::BasicBinaryOperator::LogicalLeftShift => {
            if left.sign() == Sign::Minus {
                return Err(ErrorDiagnostic::new("Cannot shift by negative", source));
            } else if let Ok(small) = u64::try_from(right) {
                left.clone() << small
            } else {
                return Err(ErrorDiagnostic::new("Cannot shift by negative", source));
            }
        }
        ast::BasicBinaryOperator::RightShift => {
            if left.sign() == Sign::Minus {
                return Err(ErrorDiagnostic::new("Cannot shift by negative", source));
            } else if let Ok(small) = u64::try_from(right) {
                left.clone() >> small
            } else {
                return Err(ErrorDiagnostic::new("Cannot shift by negative", source));
            }
        }
        ast::BasicBinaryOperator::LogicalRightShift => {
            return Err(ErrorDiagnostic::new(
                "Cannot perform logical right shift on unspecialized integer",
                source,
            ));
        }
    };

    Ok(Resolved::from_type(UnaliasedType(
        ctx.alloc(TypeKind::IntegerLiteral(result).at(source)),
    )))
}

pub fn resolve_basic_binary_operator<'env>(
    ast_operator: &ast::BasicBinaryOperator,
    ty: UnaliasedType<'env>,
    source: Source,
) -> Result<BasicBinaryOperator, ErrorDiagnostic> {
    let resolved_operator = match ast_operator {
        ast::BasicBinaryOperator::Add => ty.0.numeric_mode().map(BasicBinaryOperator::Add),
        ast::BasicBinaryOperator::Subtract => {
            ty.0.numeric_mode().map(BasicBinaryOperator::Subtract)
        }
        ast::BasicBinaryOperator::Multiply => {
            ty.0.numeric_mode().map(BasicBinaryOperator::Multiply)
        }
        ast::BasicBinaryOperator::Divide => {
            float_or_sign_lax_from_type(ty, false).map(BasicBinaryOperator::Divide)
        }
        ast::BasicBinaryOperator::Modulus => {
            float_or_sign_lax_from_type(ty, false).map(BasicBinaryOperator::Modulus)
        }
        ast::BasicBinaryOperator::Equals => {
            float_or_integer_from_type(ty, true).map(BasicBinaryOperator::Equals)
        }
        ast::BasicBinaryOperator::NotEquals => {
            float_or_integer_from_type(ty, true).map(BasicBinaryOperator::NotEquals)
        }
        ast::BasicBinaryOperator::LessThan => {
            float_or_sign_lax_from_type(ty, false).map(BasicBinaryOperator::LessThan)
        }
        ast::BasicBinaryOperator::LessThanEq => {
            float_or_sign_lax_from_type(ty, false).map(BasicBinaryOperator::LessThanEq)
        }
        ast::BasicBinaryOperator::GreaterThan => {
            float_or_sign_lax_from_type(ty, false).map(BasicBinaryOperator::GreaterThan)
        }
        ast::BasicBinaryOperator::GreaterThanEq => {
            float_or_sign_lax_from_type(ty, false).map(BasicBinaryOperator::GreaterThanEq)
        }
        ast::BasicBinaryOperator::BitwiseAnd => {
            (ty.0.kind.is_integer() || ty.0.kind.is_c_integer() || ty.0.kind.is_boolean())
                .then_some(BasicBinaryOperator::BitwiseAnd)
        }
        ast::BasicBinaryOperator::BitwiseOr => {
            (ty.0.kind.is_integer() || ty.0.kind.is_c_integer() || ty.0.kind.is_boolean())
                .then_some(BasicBinaryOperator::BitwiseOr)
        }
        ast::BasicBinaryOperator::BitwiseXor => (ty.0.kind.is_integer()
            || ty.0.kind.is_c_integer())
        .then_some(BasicBinaryOperator::BitwiseXor),
        ast::BasicBinaryOperator::LeftShift | ast::BasicBinaryOperator::LogicalLeftShift => {
            (ty.0.kind.is_integer() || ty.0.kind.is_c_integer())
                .then_some(BasicBinaryOperator::LogicalLeftShift)
        }
        ast::BasicBinaryOperator::RightShift => match ty.0.kind {
            TypeKind::BitInteger(_, sign) => Some(BasicBinaryOperator::ArithmeticRightShift(
                SignOrIndeterminate::Sign(sign),
            )),
            TypeKind::CInteger(c_integer, sign) => Some(if let Some(sign) = sign {
                BasicBinaryOperator::ArithmeticRightShift(SignOrIndeterminate::Sign(sign))
            } else {
                BasicBinaryOperator::ArithmeticRightShift(SignOrIndeterminate::Indeterminate(
                    c_integer,
                ))
            }),
            _ => None,
        },
        ast::BasicBinaryOperator::LogicalRightShift => (ty.0.kind.is_integer()
            || ty.0.kind.is_c_integer())
        .then_some(BasicBinaryOperator::LogicalRightShift),
    };

    resolved_operator.ok_or_else(|| {
        ErrorDiagnostic::new(
            format!("Cannot perform '{}' on '{}'", ast_operator, ty),
            source,
        )
    })
}

fn float_or_sign_lax_from_type<'env>(
    unified_type: UnaliasedType<'env>,
    allow_on_bools: bool,
) -> Option<FloatOrSignLax> {
    match &unified_type.0.kind {
        TypeKind::Boolean if allow_on_bools => Some(FloatOrSignLax::Integer(IntegerSign::Unsigned)),
        TypeKind::BitInteger(_, sign) => Some(FloatOrSignLax::Integer(*sign)),
        TypeKind::CInteger(c_integer, sign) => {
            if let Some(sign) = sign {
                Some(FloatOrSignLax::Integer(*sign))
            } else {
                Some(FloatOrSignLax::IndeterminateInteger(*c_integer))
            }
        }
        TypeKind::Floating(_) => Some(FloatOrSignLax::Float),
        _ => None,
    }
}

fn float_or_integer_from_type<'env>(
    unified_type: UnaliasedType<'env>,
    allow_on_bools: bool,
) -> Option<FloatOrInteger> {
    match &unified_type.0.kind {
        TypeKind::Boolean if allow_on_bools => Some(FloatOrInteger::Integer),
        TypeKind::BitInteger(..) | TypeKind::CInteger(..) => Some(FloatOrInteger::Integer),
        TypeKind::Floating(_) => Some(FloatOrInteger::Float),
        _ => None,
    }
}
