use crate::{
    ast::{self, IntegerSign, Source},
    c::{
        parser::{
            error::ParseErrorKind,
            expr::{Expr, ExprKind},
            ParseError,
        },
        token::Integer,
    },
    resolved::IntegerLiteralBits,
};

pub fn translate_expr(expr: &Expr) -> Result<ast::Expr, ParseError> {
    Ok(match &expr.kind {
        ExprKind::Integer(integer) => translate_expr_integer(integer, expr.source)?,
        ExprKind::Float(_, _) => todo!(),
        ExprKind::StringLiteral(_, _) => todo!(),
        ExprKind::Boolean(_) => todo!(),
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
        ExprKind::CompoundLiteral(_) => todo!(),
    })
}

fn translate_expr_integer(integer: &Integer, source: Source) -> Result<ast::Expr, ParseError> {
    let ast_integer = match integer {
        Integer::Int(value) => ast::Integer::Known(
            IntegerLiteralBits::Bits32,
            IntegerSign::Signed,
            (*value).into(),
        ),
        Integer::UnsignedInt(value) => ast::Integer::Known(
            IntegerLiteralBits::Bits32,
            IntegerSign::Unsigned,
            (*value).into(),
        ),
        Integer::Long(value) | Integer::LongLong(value) => ast::Integer::Known(
            IntegerLiteralBits::Bits64,
            IntegerSign::Signed,
            (*value).into(),
        ),
        Integer::UnsignedLong(value) | Integer::UnsignedLongLong(value) => ast::Integer::Known(
            IntegerLiteralBits::Bits64,
            IntegerSign::Unsigned,
            (*value).into(),
        ),
    };

    Ok(ast::ExprKind::Integer(ast_integer).at(source))
}
