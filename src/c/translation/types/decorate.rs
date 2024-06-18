use crate::{
    ast::{FixedArray, FunctionPointer, Source, Type, TypeKind},
    c::{
        parser::{ArrayQualifier, FunctionQualifier, ParseError, Pointer},
        translate_expr,
    },
};

pub fn decorate_pointer(
    ast_type: Type,
    pointer: &Pointer,
    source: Source,
) -> Result<Type, ParseError> {
    if !pointer.type_qualifiers.is_empty() {
        eprintln!("warning: ignoring pointer type qualifiers");
    }

    Ok(Type::new(TypeKind::Pointer(Box::new(ast_type)), source))
}

pub fn decorate_array(
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

    if for_parameter {
        todo!("array get_name_and_type for parameter");
    } else {
        if let Some(count) = &array.expression {
            Ok(Type::new(
                TypeKind::FixedArray(Box::new(FixedArray {
                    ast_type,
                    count: translate_expr(count)?,
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
    Ok(TypeKind::FunctionPointer(FunctionPointer {
        parameters: function.parameters.clone(),
        return_type: Box::new(ast_type),
        is_cstyle_variadic: function.is_cstyle_variadic,
    })
    .at(source))
}
