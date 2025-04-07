mod eval;
mod expr;
mod function;
mod parameters;
pub mod types;

use self::types::get_name_and_type;
pub use self::{expr::translate_expr, function::declare_function};
use crate::{CFileType, parse::ParseError};
use ast::{AstFile, TypeParams};
use attributes::{Exposure, SymbolOwnership};
use c_ast::{Attribute, CTypedef, DeclarationSpecifiers, Declarator, StorageClassSpecifier};
use diagnostics::{Diagnostics, WarningDiagnostic};
use std::collections::HashMap;

pub struct TranslateCtx<'ast, 'typedefs, 'diagnostics, 'source_files> {
    pub ast_file: &'ast mut AstFile,
    pub typedefs: &'typedefs mut HashMap<String, CTypedef>,
    pub diagnostics: &'diagnostics Diagnostics<'source_files>,
}

impl<'ast, 'typedefs, 'diagnostics, 'source_files>
    TranslateCtx<'ast, 'typedefs, 'diagnostics, 'source_files>
{
    pub fn new<'input>(
        ast_file: &'ast mut AstFile,
        typedefs: &'typedefs mut HashMap<String, CTypedef>,
        diagnostics: &'diagnostics Diagnostics<'source_files>,
    ) -> Self {
        Self {
            ast_file,
            typedefs,
            diagnostics,
        }
    }
}

pub fn declare_named_declaration(
    ctx: &mut TranslateCtx,
    declarator: &Declarator,
    _attribute_specifiers: &[Attribute],
    declaration_specifiers: &DeclarationSpecifiers,
    c_file_type: CFileType,
) -> Result<(), ParseError> {
    let decl_info = get_name_and_type(ctx, declarator, declaration_specifiers, false)?;

    if let Some(StorageClassSpecifier::Typedef) = decl_info.specifiers.storage_class {
        if let Some(function_specifier) = decl_info.specifiers.function_specifier {
            ctx.diagnostics.push(WarningDiagnostic::new(
                format!(
                    "Function specifier '{}' does nothing on typedef",
                    function_specifier.as_str()
                ),
                declarator.source,
            ));
        }

        ctx.ast_file.type_aliases.push(ast::TypeAlias {
            name: decl_info.name.clone(),
            params: TypeParams::default(),
            value: decl_info.ast_type.clone(),
            source: declarator.source,
            privacy: c_file_type.privacy(),
        });

        ctx.typedefs.insert(
            decl_info.name,
            CTypedef {
                ast_type: decl_info.ast_type,
            },
        );
        return Ok(());
    }

    if let None | Some(StorageClassSpecifier::Extern) = decl_info.specifiers.storage_class {
        let is_foreign = decl_info.specifiers.storage_class.is_some();

        if let Some(function_specifier) = decl_info.specifiers.function_specifier {
            ctx.diagnostics.push(WarningDiagnostic::new(
                format!(
                    "Function specifier '{}' on functions is not respected yet",
                    function_specifier.as_str()
                ),
                declarator.source,
            ));
        }

        let ownership = if is_foreign {
            SymbolOwnership::Reference
        } else {
            SymbolOwnership::Owned(Exposure::Exposed)
        };

        ctx.ast_file.global_variables.push(ast::GlobalVar {
            name: decl_info.name,
            ast_type: decl_info.ast_type,
            source: declarator.source,
            is_thread_local: decl_info.specifiers.is_thread_local,
            privacy: c_file_type.privacy(),
            ownership,
        });
        return Ok(());
    }

    todo!(
        "declare_named_declaration unimplemented for non-typedef - {:#?}",
        declarator
    )
}
