use crate::{parse::ParseError, translate::TranslateCtx, translate_expr};
use c_ast::{ArrayQualifier, FunctionQualifier, Pointer};
use diagnostics::{Diagnostics, WarningDiagnostic};
use source_files::Source;

pub fn decorate_pointer(
    ast_type: ast::Type,
    pointer: &Pointer,
    source: Source,
    diagnostics: &Diagnostics,
) -> Result<ast::Type, ParseError> {
    if !pointer.type_qualifiers.is_empty() {
        diagnostics.push(WarningDiagnostic::new(
            "Ignoring pointer type qualifiers",
            source,
        ))
    }

    Ok(ast::Type::new(
        ast::TypeKind::Ptr(Box::new(ast_type)),
        source,
    ))
}

pub fn decorate_array(
    ctx: &mut TranslateCtx,
    ast_type: ast::Type,
    array: &ArrayQualifier,
    for_parameter: bool,
    source: Source,
) -> Result<ast::Type, ParseError> {
    if !array.type_qualifiers.is_empty() {
        todo!("array type qualifiers not supported yet");
    }

    if array.is_static {
        todo!("array static");
    }

    if array.is_param_vla {
        todo!("array get_name_and_type VLA");
    }

    #[allow(clippy::collapsible_else_if)]
    if for_parameter {
        todo!("array get_name_and_type for parameter");
    } else {
        if let Some(count) = &array.expression {
            Ok(ast::Type::new(
                ast::TypeKind::FixedArray(Box::new(ast::FixedArray {
                    ast_type,
                    count: translate_expr(ctx, count)?,
                })),
                source,
            ))
        } else {
            todo!("array get_name_and_type array non-parameter vla?");
        }
    }
}

pub fn decorate_function(
    ast_type: ast::Type,
    function: &FunctionQualifier,
    source: Source,
) -> Result<ast::Type, ParseError> {
    Ok(ast::TypeKind::FuncPtr(ast::FuncPtr {
        parameters: function.params.clone(),
        return_type: Box::new(ast_type),
        is_cstyle_variadic: function.is_cstyle_variadic,
    })
    .at(source))
}
