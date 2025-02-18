use crate::{
    ast::{self, AstFile},
    c::{
        parser::{expr::Caster, CTypedef, ParseError},
        translation::types::{build_type_specifier_qualifier, TypeBaseBuilder},
    },
    diagnostics::Diagnostics,
};
use std::collections::HashMap;

pub fn get_caster_type(
    ast_file: &mut AstFile,
    typedefs: &HashMap<String, CTypedef>,
    caster: &Caster,
    diagnostics: &Diagnostics,
) -> Result<ast::Type, ParseError> {
    let mut builder = TypeBaseBuilder::new(caster.source);

    if !caster.specializer_qualifiers.attributes.is_empty() {
        todo!("attributes not supported yet for caster specializer qualifiers");
    }

    for tsq in caster
        .specializer_qualifiers
        .type_specifier_qualifiers
        .iter()
    {
        build_type_specifier_qualifier(ast_file, &mut builder, typedefs, tsq, diagnostics)?;
    }

    if let Some(_abstract_declarator) = &caster.abstract_declarator {
        todo!("abstract declarator for caster not supported yet");
    }

    let base = builder.build()?;

    if base.storage_class.is_some() {
        return Err(ParseError::message(
            "Storage class specifier cannot be used on cast",
            caster.source,
        ));
    }

    if base.function_specifier.is_some() {
        return Err(ParseError::message(
            "Storage class specifier cannot be used on cast",
            caster.source,
        ));
    }

    Ok(base.ast_type)
}
