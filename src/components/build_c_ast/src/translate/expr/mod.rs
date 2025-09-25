mod caster;
mod compound_literal;
mod integer;
mod string;

use self::{
    compound_literal::translate_compound_literal, integer::translate_expr_integer,
    string::translate_expr_string,
};
use super::TranslateCtx;
use crate::parse::ParseError;
use ast::Language;
use c_ast::{BinaryOperator, Expr, ExprKind};
use smallvec::smallvec;

pub fn translate_expr(ctx: &mut TranslateCtx, expr: &Expr) -> Result<ast::Expr, ParseError> {
    Ok(match &expr.kind {
        ExprKind::Integer(integer) => translate_expr_integer(integer, expr.source)?,
        ExprKind::Float(_, _) => todo!("translate_expr float"),
        ExprKind::StringLiteral(encoding, content) => {
            translate_expr_string(encoding, content, expr.source)?
        }
        ExprKind::Bool(x) => ast::ExprKind::Boolean(*x).at(expr.source),
        ExprKind::Nullptr => todo!("translate_expr nullptr"),
        ExprKind::Character(_, _) => todo!("translate_expr character"),
        ExprKind::Compound(_) => todo!("translate_expr compound"),
        ExprKind::BinaryOperation(operation) => {
            let left = translate_expr(ctx, &operation.left)?;
            let right = translate_expr(ctx, &operation.right)?;

            // TODO: Perfrom usual arithmetic conversions or integer promotions depending on
            // operator
            // TODO: Array-to-Pointer and Function-to-Pointer conversions may also apply
            let operator: ast::BinaryOperator = match operation.operator {
                BinaryOperator::LogicalOr => ast::ShortCircuitingBinaryOperator::Or.into(),
                BinaryOperator::LogicalAnd => ast::ShortCircuitingBinaryOperator::And.into(),
                BinaryOperator::InclusiveOr => ast::BasicBinaryOperator::BitwiseOr.into(),
                BinaryOperator::ExclusiveOr => ast::BasicBinaryOperator::BitwiseXor.into(),
                BinaryOperator::BitwiseAnd => ast::BasicBinaryOperator::BitwiseAnd.into(),
                BinaryOperator::Equals => ast::BasicBinaryOperator::Equals.into(),
                BinaryOperator::NotEquals => ast::BasicBinaryOperator::NotEquals.into(),
                BinaryOperator::LessThan => ast::BasicBinaryOperator::LessThan.into(),
                BinaryOperator::GreaterThan => ast::BasicBinaryOperator::GreaterThan.into(),
                BinaryOperator::LessThanEq => ast::BasicBinaryOperator::LessThanEq.into(),
                BinaryOperator::GreaterThanEq => ast::BasicBinaryOperator::GreaterThanEq.into(),
                BinaryOperator::LeftShift => ast::BasicBinaryOperator::LeftShift.into(),
                BinaryOperator::RightShift => ast::BasicBinaryOperator::RightShift.into(),
                BinaryOperator::Add => ast::BasicBinaryOperator::Add.into(),
                BinaryOperator::Subtract => ast::BasicBinaryOperator::Subtract.into(),
                BinaryOperator::Multiply => ast::BasicBinaryOperator::Multiply.into(),
                BinaryOperator::Divide => ast::BasicBinaryOperator::Divide.into(),
                BinaryOperator::Modulus => ast::BasicBinaryOperator::Modulus.into(),
                BinaryOperator::Assign => todo!("translate_expr assign"),
                BinaryOperator::AddAssign => todo!("translate_expr add assign"),
                BinaryOperator::SubtractAssign => todo!("translate_expr subtract assign"),
                BinaryOperator::MultiplyAssign => todo!("translate_expr multiply assign"),
                BinaryOperator::DivideAssign => todo!("translate_expr divide assign"),
                BinaryOperator::ModulusAssign => todo!("translate_expr modulus assign"),
                BinaryOperator::LeftShiftAssign => todo!("translate_expr left shift assign"),
                BinaryOperator::RightShiftAssign => todo!("translate_expr right shift assign"),
                BinaryOperator::BitAndAssign => todo!("translate_expr bitwise-and assign"),
                BinaryOperator::BitXorAssign => todo!("translate_expr bitwise-xor assign"),
                BinaryOperator::BitOrAssign => todo!("translate_expr bitwise-ox assign"),
            };

            match operator {
                ast::BinaryOperator::Basic(operator) => {
                    ast::ExprKind::BasicBinaryOperation(Box::new(ast::BasicBinaryOperation {
                        operator,
                        left,
                        right,
                        language: Language::C,
                    }))
                }
                ast::BinaryOperator::ShortCircuiting(operator) => {
                    ast::ExprKind::ShortCircuitingBinaryOperation(Box::new(
                        ast::ShortCircuitingBinaryOperation {
                            operator,
                            left,
                            right,
                            conform_behavior: ast::ConformBehavior::C,
                        },
                    ))
                }
            }
            .at(expr.source)
        }
        ExprKind::Ternary(_) => todo!(),
        ExprKind::Cast(_) => todo!(),
        ExprKind::Subscript(_) => todo!(),
        ExprKind::Field(_) => todo!(),
        ExprKind::PreIncrement(_) => todo!(),
        ExprKind::PreDecrement(_) => todo!(),
        ExprKind::PostIncrement(_) => todo!(),
        ExprKind::PostDecrement(_) => todo!(),
        ExprKind::Identifier(name) => {
            return Ok(ast::ExprKind::Variable(ast::NamePath::new(smallvec![
                name.clone().into_boxed_str()
            ]))
            .at(expr.source));
        }
        ExprKind::EnumConstant(_, _) => todo!(),
        ExprKind::CompoundLiteral(compound_literal) => {
            translate_compound_literal(ctx, &compound_literal, expr.source)?
        }
        ExprKind::AddressOf(inner) => {
            ast::ExprKind::UnaryOperation(Box::new(ast::UnaryOperation {
                operator: ast::UnaryOperator::AddressOf,
                inner: translate_expr(ctx, inner)?,
            }))
            .at(expr.source)
        }
        ExprKind::Dereference(inner) => {
            ast::ExprKind::UnaryOperation(Box::new(ast::UnaryOperation {
                operator: ast::UnaryOperator::Dereference,
                inner: translate_expr(ctx, inner)?,
            }))
            .at(expr.source)
        }
        ExprKind::Negate(inner) => {
            // TODO: Perform integer promotion
            ast::ExprKind::UnaryOperation(Box::new(ast::UnaryOperation {
                operator: ast::UnaryOperator::Math(ast::UnaryMathOperator::Negate),
                inner: translate_expr(ctx, inner)?,
            }))
            .at(expr.source)
        }
        ExprKind::BitComplement(inner) => {
            // TODO: Perform integer promotion
            ast::ExprKind::UnaryOperation(Box::new(ast::UnaryOperation {
                operator: ast::UnaryOperator::Math(ast::UnaryMathOperator::BitComplement),
                inner: translate_expr(ctx, inner)?,
            }))
            .at(expr.source)
        }
        ExprKind::Not(inner) => ast::ExprKind::UnaryOperation(Box::new(ast::UnaryOperation {
            operator: ast::UnaryOperator::Math(ast::UnaryMathOperator::Not),
            inner: translate_expr(ctx, inner)?,
        }))
        .at(expr.source),
        ExprKind::Call(target, c_args) => {
            eprintln!("warning: c function call expression cannot call expression yet");

            let ExprKind::Identifier(target) = &target.as_ref().kind else {
                return Err(ParseError::message(
                    "Calling the result of expressions is not supported yet",
                    expr.source,
                ));
            };

            let args = c_args
                .iter()
                .map(|c_arg| translate_expr(ctx, c_arg))
                .collect::<Result<Vec<ast::Expr>, ParseError>>()?;

            ast::ExprKind::Call(Box::new(ast::Call {
                name_path: ast::NamePath::new(smallvec![target.clone().into_boxed_str()]),
                args,
                expected_to_return: None,
                generics: vec![],
                using: vec![],
            }))
            .at(expr.source)
        }
        ExprKind::SizeOf(ty, mode) => {
            ast::ExprKind::SizeOf(Box::new(ty.clone()), *mode).at(expr.source)
        }
        ExprKind::SizeOfValue(value, mode) => {
            ast::ExprKind::SizeOfValue(Box::new(translate_expr(ctx, value)?), *mode).at(expr.source)
        }
        ExprKind::AlignOf(_) => todo!("translate_expr AlignOf"),
        ExprKind::IntegerPromote(value) => {
            ast::ExprKind::IntegerPromote(Box::new(translate_expr(ctx, value)?)).at(expr.source)
        }
    })
}
