use super::{build_type_specifier_qualifier, TypeBase, TypeBaseBuilder};
use crate::{
    ast::{self, Source},
    c::parser::{CTypedef, DeclarationSpecifierKind, DeclarationSpecifiers, ParseError},
    diagnostics::Diagnostics,
};
use std::collections::HashMap;

pub fn get_type_base(
    ast_file: &mut ast::File,
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
            DeclarationSpecifierKind::Auto => {
                return Err(ParseError::message(
                    "'auto' not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::Constexpr => {
                return Err(ParseError::message(
                    "'constexpr' not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::Extern => {
                return Err(ParseError::message(
                    "'extern' not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::Register => {
                return Err(ParseError::message(
                    "'register' declaration specifier not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::Static => {
                return Err(ParseError::message(
                    "'static' declaration specifier not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::ThreadLocal => {
                return Err(ParseError::message(
                    "'thread_local' declaration specifier not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::Typedef => builder.is_typedef = true,
            DeclarationSpecifierKind::Inline => {
                return Err(ParseError::message(
                    "'inline' declaration specifier not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::Noreturn => {
                return Err(ParseError::message(
                    "'_Noreturn' declaration specifier not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::TypeSpecifierQualifier(tsq) => {
                build_type_specifier_qualifier(ast_file, &mut builder, typedefs, tsq, diagnostics)?
            }
        }
    }

    builder.build()
}
