use crate::{
    ast::{self, ConformBehavior, FillBehavior, Source},
    c::{
        parser::{
            expr::{CompoundLiteral, Initializer},
            ParseError,
        },
        translate_expr,
    },
};

pub fn translate_compound_literal(
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
