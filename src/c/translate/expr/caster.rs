use super::TranslateCtx;
use crate::{
    ast::{self},
    c::{
        ast::expr::Caster,
        parser::ParseError,
        translate::types::{build_type_specifier_qualifier, TypeBaseBuilder},
    },
};

pub fn get_caster_type(ctx: &mut TranslateCtx, caster: &Caster) -> Result<ast::Type, ParseError> {
    let mut builder = TypeBaseBuilder::new(caster.source);

    if !caster.specializer_qualifiers.attributes.is_empty() {
        todo!("attributes not supported yet for caster specializer qualifiers");
    }

    for tsq in caster
        .specializer_qualifiers
        .type_specifier_qualifiers
        .iter()
    {
        build_type_specifier_qualifier(ctx, &mut builder, tsq)?;
    }

    if let Some(_abstract_declarator) = &caster.abstract_declarator {
        todo!("abstract declarator for caster not supported yet");
    }

    let base = builder.build()?;

    if base.specifiers.storage_class.is_some() {
        return Err(ParseError::message(
            "Storage class specifier cannot be used on cast",
            caster.source,
        ));
    }

    if base.specifiers.function_specifier.is_some() {
        return Err(ParseError::message(
            "Storage class specifier cannot be used on cast",
            caster.source,
        ));
    }

    Ok(base.ast_type)
}
