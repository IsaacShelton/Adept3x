mod eval;
mod expr;
mod function;
mod parameters;
mod types;

use self::types::get_name_and_type;
pub use self::{expr::translate_expr, function::declare_function};
use crate::{
    asg::TypeParams,
    ast::{self, AstFile, Privacy},
    c::parser::{CTypedef, DeclarationSpecifiers, Declarator, ParseError, StorageClassSpecifier},
    diagnostics::{Diagnostics, WarningDiagnostic},
};
use std::collections::HashMap;

pub fn declare_named_declaration(
    ast_file: &mut AstFile,
    declarator: &Declarator,
    _attribute_specifiers: &[()],
    declaration_specifiers: &DeclarationSpecifiers,
    typedefs: &mut HashMap<String, CTypedef>,
    diagnostics: &Diagnostics,
) -> Result<(), ParseError> {
    let (name, ast_type, storage_class, function_specifier) = get_name_and_type(
        ast_file,
        typedefs,
        declarator,
        declaration_specifiers,
        false,
        diagnostics,
    )?;

    if let Some(StorageClassSpecifier::Typedef) = storage_class {
        if let Some(function_specifier) = function_specifier {
            diagnostics.push(WarningDiagnostic::new(
                format!(
                    "Function specifier '{}' does nothing on typedef",
                    function_specifier.as_str()
                ),
                declarator.source,
            ));
        }

        ast_file.type_aliases.push(ast::TypeAlias {
            name: name.clone(),
            params: TypeParams::default(),
            value: ast_type.clone(),
            source: declarator.source,
            privacy: Privacy::Public,
        });

        typedefs.insert(name, CTypedef { ast_type });
        return Ok(());
    }

    todo!(
        "declare_named_declaration unimplemented for non-typedef - {:#?}",
        declarator
    )
}
