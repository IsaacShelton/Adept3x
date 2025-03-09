use super::TranslateCtx;
use crate::{
    ast::{self, FillBehavior, Language},
    c::{
        ast::expr::{CompoundLiteral, Initializer},
        parser::ParseError,
        translate::expr::caster::get_caster_type,
        translate_expr,
    },
    source_files::Source,
};

pub fn translate_compound_literal(
    ctx: &mut TranslateCtx,
    compound_literal: &CompoundLiteral,
    source: Source,
) -> Result<ast::Expr, ParseError> {
    let ast_type = get_caster_type(ctx, &compound_literal.caster)?;
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
            Initializer::Expression(expr) => translate_expr(ctx, expr)?,
            Initializer::BracedInitializer(_) => {
                todo!("nested brace initializer for translate_compound_literal")
            }
        };

        fields.push(ast::FieldInitializer { name: None, value });
    }

    Ok(ast::ExprKind::StructLiteral(Box::new(ast::StructLiteral {
        ast_type,
        fields,
        fill_behavior: FillBehavior::Zeroed,
        language: Language::C,
    }))
    .at(source))
}
