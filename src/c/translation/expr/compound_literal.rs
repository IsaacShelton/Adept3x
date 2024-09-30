use crate::{
    ast::{self, AstFile, FillBehavior, Language},
    c::{
        parser::{
            expr::{CompoundLiteral, Initializer},
            CTypedef, ParseError,
        },
        translate_expr,
        translation::expr::caster::get_caster_type,
    },
    diagnostics::Diagnostics,
    source_files::Source,
};
use std::collections::HashMap;

pub fn translate_compound_literal(
    ast_file: &mut AstFile,
    typedefs: &HashMap<String, CTypedef>,
    compound_literal: &CompoundLiteral,
    source: Source,
    diagnostics: &Diagnostics,
) -> Result<ast::Expr, ParseError> {
    let ast_type = get_caster_type(ast_file, typedefs, &compound_literal.caster, diagnostics)?;
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
            Initializer::Expression(expr) => translate_expr(ast_file, typedefs, expr, diagnostics)?,
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
