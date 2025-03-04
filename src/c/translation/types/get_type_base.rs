use super::{build_type_specifier_qualifier, TypeBase, TypeBaseBuilder};
use crate::{
    ast::AstFile,
    c::{
        ast::{CTypedef, DeclarationSpecifierKind, DeclarationSpecifiers},
        parser::ParseError,
    },
    diagnostics::Diagnostics,
    source_files::Source,
};
use std::collections::HashMap;

pub fn get_type_base(
    ast_file: &mut AstFile,
    typedefs: &HashMap<String, CTypedef>,
    declaration_specifiers: &DeclarationSpecifiers,
    parent_source: Source,
    diagnostics: &Diagnostics,
) -> Result<TypeBase, ParseError> {
    let mut builder = TypeBaseBuilder::new(parent_source);

    if !declaration_specifiers.attributes.is_empty() {
        return Err(ParseError::message(
            "Attributes on declaration specifiers not supported yet",
            parent_source,
        ));
    }

    for specifier in declaration_specifiers.specifiers.iter() {
        match &specifier.kind {
            DeclarationSpecifierKind::StorageClassSpecifier(storage_class) => {
                builder.storage_class = Some(*storage_class);
            }
            DeclarationSpecifierKind::FunctionSpecifier(function_specifier) => {
                builder.function_specifier = Some(*function_specifier);
            }

            DeclarationSpecifierKind::TypeSpecifierQualifier(tsq) => {
                build_type_specifier_qualifier(ast_file, &mut builder, typedefs, tsq, diagnostics)?
            }
        }
    }

    builder.build()
}
