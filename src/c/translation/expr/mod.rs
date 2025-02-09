mod caster;
mod compound_literal;
mod integer;
mod string;

use self::{
    compound_literal::translate_compound_literal, integer::translate_expr_integer,
    string::translate_expr_string,
};
use crate::{
    ast::{self, AstFile},
    c::parser::{
        error::ParseErrorKind,
        expr::{BinaryOperator, Expr, ExprKind},
        CTypedef, ParseError,
    },
    diagnostics::Diagnostics,
};
use std::collections::HashMap;

pub fn translate_expr(
    ast_file: &mut AstFile,
    typedefs: &HashMap<String, CTypedef>,
    expr: &Expr,
    diagnostics: &Diagnostics,
) -> Result<ast::Expr, ParseError> {
    Ok(match &expr.kind {
        ExprKind::Integer(integer) => translate_expr_integer(integer, expr.source)?,
        ExprKind::Float(_, _) => todo!(),
        ExprKind::StringLiteral(encoding, content) => {
            translate_expr_string(encoding, content, expr.source)?
        }
        ExprKind::Bool(x) => ast::ExprKind::Boolean(*x).at(expr.source),
        ExprKind::Nullptr => todo!(),
        ExprKind::Character(_, _) => todo!(),
        ExprKind::Compound(_) => todo!(),
        ExprKind::BinaryOperation(operation) => {
            let left = translate_expr(ast_file, typedefs, &operation.left, diagnostics)?;
            let right = translate_expr(ast_file, typedefs, &operation.right, diagnostics)?;

            let op: ast::BinaryOperator = match operation.operator {
                BinaryOperator::LogicalOr => todo!(),
                BinaryOperator::LogicalAnd => todo!(),
                BinaryOperator::InclusiveOr => todo!(),
                BinaryOperator::ExclusiveOr => todo!(),
                BinaryOperator::BitwiseAnd => todo!(),
                BinaryOperator::Equals => todo!(),
                BinaryOperator::NotEquals => todo!(),
                BinaryOperator::LessThan => todo!(),
                BinaryOperator::GreaterThan => todo!(),
                BinaryOperator::LessThanEq => todo!(),
                BinaryOperator::GreaterThanEq => todo!(),
                BinaryOperator::LeftShift => todo!(),
                BinaryOperator::RightShift => todo!(),
                BinaryOperator::Add => ast::BasicBinaryOperator::Add.into(),
                BinaryOperator::Subtract => ast::BasicBinaryOperator::Subtract.into(),
                BinaryOperator::Multiply => ast::BasicBinaryOperator::Multiply.into(),
                BinaryOperator::Divide => ast::BasicBinaryOperator::Divide.into(),
                BinaryOperator::Modulus => ast::BasicBinaryOperator::Modulus.into(),
                BinaryOperator::Assign => todo!(),
                BinaryOperator::AddAssign => todo!(),
                BinaryOperator::SubtractAssign => todo!(),
                BinaryOperator::MultiplyAssign => todo!(),
                BinaryOperator::DivideAssign => todo!(),
                BinaryOperator::ModulusAssign => todo!(),
                BinaryOperator::LeftShiftAssign => todo!(),
                BinaryOperator::RightShiftAssign => todo!(),
                BinaryOperator::BitAndAssign => todo!(),
                BinaryOperator::BitXorAssign => todo!(),
                BinaryOperator::BitOrAssign => todo!(),
            };

            match op {
                ast::BinaryOperator::Basic(operator) => {
                    ast::ExprKind::BasicBinaryOperation(Box::new(ast::BasicBinaryOperation {
                        operator,
                        left,
                        right,
                    }))
                }
                ast::BinaryOperator::ShortCircuiting(operator) => {
                    ast::ExprKind::ShortCircuitingBinaryOperation(Box::new(
                        ast::ShortCircuitingBinaryOperation {
                            operator,
                            left,
                            right,
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
        ExprKind::PostIncrement(_) => todo!(),
        ExprKind::PostDecrement(_) => todo!(),
        ExprKind::Identifier(name) => {
            return Err(ParseErrorKind::UndefinedVariable(name.into()).at(expr.source));
        }
        ExprKind::EnumConstant(_, _) => todo!(),
        ExprKind::CompoundLiteral(compound_literal) => translate_compound_literal(
            ast_file,
            typedefs,
            compound_literal,
            expr.source,
            diagnostics,
        )?,
        ExprKind::AddressOf(inner) => {
            ast::ExprKind::UnaryOperation(Box::new(ast::UnaryOperation {
                operator: ast::UnaryOperator::AddressOf,
                inner: translate_expr(ast_file, typedefs, inner, diagnostics)?,
            }))
            .at(expr.source)
        }
        ExprKind::Dereference(inner) => {
            ast::ExprKind::UnaryOperation(Box::new(ast::UnaryOperation {
                operator: ast::UnaryOperator::Dereference,
                inner: translate_expr(ast_file, typedefs, inner, diagnostics)?,
            }))
            .at(expr.source)
        }
        ExprKind::Negate(inner) => ast::ExprKind::UnaryOperation(Box::new(ast::UnaryOperation {
            operator: ast::UnaryOperator::Math(ast::UnaryMathOperator::Negate),
            inner: translate_expr(ast_file, typedefs, inner, diagnostics)?,
        }))
        .at(expr.source),
        ExprKind::BitComplement(inner) => {
            ast::ExprKind::UnaryOperation(Box::new(ast::UnaryOperation {
                operator: ast::UnaryOperator::Math(ast::UnaryMathOperator::BitComplement),
                inner: translate_expr(ast_file, typedefs, inner, diagnostics)?,
            }))
            .at(expr.source)
        }
        ExprKind::Not(inner) => ast::ExprKind::UnaryOperation(Box::new(ast::UnaryOperation {
            operator: ast::UnaryOperator::Math(ast::UnaryMathOperator::Not),
            inner: translate_expr(ast_file, typedefs, inner, diagnostics)?,
        }))
        .at(expr.source),
    })
}
