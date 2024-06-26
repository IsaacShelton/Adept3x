use crate::{
    ast::{self, ConformBehavior, FillBehavior, Source},
    c::{
        parser::{
            expr::{CompoundLiteral, Initializer},
            CTypedef, ParseError,
        },
        translate_expr,
        translation::expr::caster::get_caster_type,
    },
};
use std::collections::HashMap;

pub fn translate_compound_literal(
    ast_file: &mut ast::File,
    typedefs: &HashMap<String, CTypedef>,
    compound_literal: &CompoundLiteral,
    source: Source,
) -> Result<ast::Expr, ParseError> {
    let ty = get_caster_type(ast_file, typedefs, &compound_literal.caster)?;
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
            Initializer::Expression(expr) => translate_expr(ast_file, typedefs, expr)?,
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
