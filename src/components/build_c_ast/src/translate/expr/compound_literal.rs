use super::{TranslateCtx, caster::get_caster_type, translate_expr};
use crate::parse::ParseError;
use c_ast::{CompoundLiteral, Initializer};
use source_files::Source;

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
        fill_behavior: ast::FillBehavior::Zeroed,
        language: ast::Language::C,
    }))
    .at(source))
}
