use crate::{
    ast::{self, ConformBehavior, FillBehavior, IntegerSign, Source},
    c::{
        encoding::Encoding,
        parser::{
            error::ParseErrorKind,
            expr::{CompoundLiteral, Expr, ExprKind, Initializer},
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
            translate_compound_literal(compound_literal, expr.source)?
        }
    })
}

fn translate_compound_literal(
    compound_literal: &CompoundLiteral,
    source: Source,
) -> Result<ast::Expr, ParseError> {
    eprintln!(
        "translate compound literal (needs type) {:#?}",
        compound_literal
    );

    let ty = ast::TypeKind::Named("struct<Color>".into()).at(compound_literal.caster.source);
    let mut fields = Vec::new();

    for init in compound_literal
        .braced_initializer
        .designated_initializers
        .iter()
    {
        if init.designation.is_some() {
            todo!("designated initializer translation");
        }

        let value = match &init.initializer {
            Initializer::Expression(expr) => translate_expr(expr)?,
            Initializer::BracedInitializer(_) => {
                todo!("nested brace initializer for translate_compound_literal")
            }
        };

        fields.push(ast::FieldInitializer { name: None, value });
    }

    Ok(
        ast::ExprKind::StructureLiteral(ty, fields, FillBehavior::Zeroed, ConformBehavior::C)
            .at(source),
    )
}

fn translate_expr_integer(integer: &Integer, source: Source) -> Result<ast::Expr, ParseError> {
    use IntegerLiteralBits::{Bits32, Bits64};
    use IntegerSign::{Signed, Unsigned};

    let ast_integer = match integer {
        Integer::Int(x) => ast::Integer::Known(Bits32, Signed, (*x).into()),
        Integer::UnsignedInt(x) => ast::Integer::Known(Bits32, Unsigned, (*x).into()),
        Integer::Long(x) | Integer::LongLong(x) => ast::Integer::Known(Bits64, Signed, (*x).into()),
        Integer::UnsignedLong(x) | Integer::UnsignedLongLong(x) => {
            ast::Integer::Known(Bits64, Unsigned, (*x).into())
        }
    };

    Ok(ast::ExprKind::Integer(ast_integer).at(source))
}

fn translate_expr_string(
    encoding: &Encoding,
    content: &str,
    source: Source,
) -> Result<ast::Expr, ParseError> {
    if let Encoding::Default = encoding {
        Ok(ast::ExprKind::String(content.into()).at(source))
    } else {
        todo!("translate non-default encoding C string")
    }
}
