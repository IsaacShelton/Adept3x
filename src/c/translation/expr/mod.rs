mod caster;
mod compound_literal;
mod integer;
mod string;

use self::{
    compound_literal::translate_compound_literal, integer::translate_expr_integer,
    string::translate_expr_string,
};
use crate::{
    ast,
    c::parser::{
        error::ParseErrorKind,
        expr::{Expr, ExprKind},
        CTypedef, ParseError,
    },
};
use std::collections::HashMap;

pub fn translate_expr(
    ast_file: &mut ast::File,
    typedefs: &HashMap<String, CTypedef>,
    expr: &Expr,
) -> Result<ast::Expr, ParseError> {
    Ok(match &expr.kind {
        ExprKind::Integer(integer) => translate_expr_integer(integer, expr.source)?,
        ExprKind::Float(_, _) => todo!(),
        ExprKind::StringLiteral(encoding, content) => {
            translate_expr_string(encoding, content, expr.source)?
        }
        ExprKind::Boolean(x) => ast::ExprKind::Boolean(*x).at(expr.source),
        ExprKind::Nullptr => todo!(),
        ExprKind::Character(_, _) => todo!(),
        ExprKind::Compound(_) => todo!(),
        ExprKind::BinaryOperation(_) => todo!(),
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
        ExprKind::CompoundLiteral(compound_literal) => {
            translate_compound_literal(ast_file, typedefs, compound_literal, expr.source)?
        }
    })
}
