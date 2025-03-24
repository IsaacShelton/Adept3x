use crate::c::ast::ParameterTypeList;

pub fn has_parameters(parameter_type_list: &ParameterTypeList) -> bool {
    let declarations = &parameter_type_list.parameter_declarations;

    if let Some(param) = declarations.first() {
        if param.core.is_nothing()
            && param.attributes.is_empty()
            && param.declaration_specifiers.attributes.is_empty()
            && param
                .declaration_specifiers
                .specifiers
                .first()
                .map_or(false, |specifier| specifier.kind.is_void())
            && param.declaration_specifiers.specifiers.len() == 1
        {
            return false; // Function has parameters `(void)`, which means no parameters
        }

        if param.core.is_nothing()
            && param.attributes.is_empty()
            && param.declaration_specifiers.is_empty()
        {
            return false; // Treat function with `()` parameters as not having any parameters
        }
    }

    !parameter_type_list.parameter_declarations.is_empty()
}
