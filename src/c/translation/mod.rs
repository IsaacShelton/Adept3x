mod eval;
mod expr;
mod function;
mod parameters;
mod types;

use self::types::get_name_and_type;
pub use self::{expr::translate_expr, function::declare_function};
use crate::{
    ast::{self, AstFile, Privacy},
    c::parser::{CTypedef, DeclarationSpecifiers, Declarator, ParseError},
    diagnostics::Diagnostics,
    name::Name,
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
    let (name, ast_type, is_typedef) = get_name_and_type(
        ast_file,
        typedefs,
        declarator,
        declaration_specifiers,
        false,
        diagnostics,
    )?;

    if is_typedef {
        ast_file.type_aliases.insert(
            Name::plain(name.clone()),
            ast::TypeAlias {
                value: ast_type.clone(),
                source: declarator.source,
                privacy: Privacy::Public,
            },
        );

        typedefs.insert(name, CTypedef { ast_type });
        return Ok(());
    }

    todo!(
        "declare_named_declaration unimplemented for non-typedef - {:#?}",
        declarator
    )
}
