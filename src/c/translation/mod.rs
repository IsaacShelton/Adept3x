mod eval;
mod expr;
mod function;
mod parameters;
mod types;

use self::types::get_name_and_type;
use crate::ast::{self, File};
use crate::c::parser::{CTypedef, DeclarationSpecifiers, Declarator, ParseError};
use std::collections::HashMap;

pub use self::expr::translate_expr;
pub use self::function::declare_function;

pub fn declare_named_declaration(
    ast_file: &mut File,
    declarator: &Declarator,
    _attribute_specifiers: &[()],
    declaration_specifiers: &DeclarationSpecifiers,
    typedefs: &mut HashMap<String, CTypedef>,
) -> Result<(), ParseError> {
    let (name, ast_type, is_typedef) = get_name_and_type(
        ast_file,
        typedefs,
        declarator,
        declaration_specifiers,
        false,
    )?;

    if is_typedef {
        ast_file.aliases.insert(
            name.to_string(),
            ast::Alias {
                value: ast_type.clone(),
                source: declarator.source,
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
