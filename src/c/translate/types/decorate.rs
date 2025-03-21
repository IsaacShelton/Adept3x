use crate::{
    ast::{FixedArray, FuncPtr, Type, TypeKind},
    c::{
        ast::{ArrayQualifier, FunctionQualifier, Pointer},
        parser::ParseError,
        translate::TranslateCtx,
        translate_expr,
    },
    diagnostics::{Diagnostics, WarningDiagnostic},
    source_files::Source,
};

pub fn decorate_pointer(
    ast_type: Type,
    pointer: &Pointer,
    source: Source,
    diagnostics: &Diagnostics,
) -> Result<Type, ParseError> {
    if !pointer.type_qualifiers.is_empty() {
        diagnostics.push(WarningDiagnostic::new(
            "Ignoring pointer type qualifiers",
            source,
        ))
    }

    Ok(Type::new(TypeKind::Ptr(Box::new(ast_type)), source))
}

pub fn decorate_array(
    ctx: &mut TranslateCtx,
    ast_type: Type,
    array: &ArrayQualifier,
    for_parameter: bool,
    source: Source,
) -> Result<Type, ParseError> {
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
            Ok(Type::new(
                TypeKind::FixedArray(Box::new(FixedArray {
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
    ast_type: Type,
    function: &FunctionQualifier,
    source: Source,
) -> Result<Type, ParseError> {
    Ok(TypeKind::FuncPtr(FuncPtr {
        parameters: function.params.clone(),
        return_type: Box::new(ast_type),
        is_cstyle_variadic: function.is_cstyle_variadic,
    })
    .at(source))
}
