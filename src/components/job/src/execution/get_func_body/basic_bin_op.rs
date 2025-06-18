use crate::{Typed, repr::TypeKind};
use diagnostics::ErrorDiagnostic;
use num_bigint::{BigInt, Sign};
use source_files::Source;

pub fn resolve_basic_binary_operation_expr_on_literals<'env>(
    operator: &ast::BasicBinaryOperator,
    left: &BigInt,
    right: &BigInt,
    source: Source,
) -> Result<Typed<'env>, ErrorDiagnostic> {
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
            return Ok(Typed::from_type(
                TypeKind::BooleanLiteral(left == right).at(source),
            ));
        }
        ast::BasicBinaryOperator::NotEquals => {
            return Ok(Typed::from_type(
                TypeKind::BooleanLiteral(left != right).at(source),
            ));
        }
        ast::BasicBinaryOperator::LessThan => {
            return Ok(Typed::from_type(
                TypeKind::BooleanLiteral(left < right).at(source),
            ));
        }
        ast::BasicBinaryOperator::LessThanEq => {
            return Ok(Typed::from_type(
                TypeKind::BooleanLiteral(left < right).at(source),
            ));
        }
        ast::BasicBinaryOperator::GreaterThan => {
            return Ok(Typed::from_type(
                TypeKind::BooleanLiteral(left > right).at(source),
            ));
        }
        ast::BasicBinaryOperator::GreaterThanEq => {
            return Ok(Typed::from_type(
                TypeKind::BooleanLiteral(left >= right).at(source),
            ));
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

    Ok(Typed::from_type(
        TypeKind::IntegerLiteral(result).at(source),
    ))
}
