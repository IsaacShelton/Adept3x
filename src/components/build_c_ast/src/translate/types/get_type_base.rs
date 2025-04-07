use super::{TypeBase, TypeBaseBuilder, build_type_specifier_qualifier};
use crate::{parse::ParseError, translate::TranslateCtx};
use c_ast::*;
use source_files::Source;

pub fn get_type_base(
    ctx: &mut TranslateCtx,
    declaration_specifiers: &DeclarationSpecifiers,
    parent_source: Source,
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
                build_type_specifier_qualifier(ctx, &mut builder, tsq)?
            }
        }
    }

    builder.build()
}
