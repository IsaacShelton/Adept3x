mod eval;
mod expr;
mod type_base;

use super::{
    error::ParseErrorKind, CTypedef, DeclarationSpecifiers, Declarator, ParameterTypeList,
    ParseError,
};
use crate::{
    ast::{self, File, Function, Parameter, Parameters},
    c::parser::{translation::type_base::get_name_and_type, ParameterDeclarationCore},
};
use std::collections::HashMap;

pub fn declare_named(
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
                name: name.clone(),
                value: ast_type.clone(),
                source: declarator.source,
            },
        );

        typedefs.insert(name, CTypedef { ast_type });
        Ok(())
    } else {
        todo!(
            "declare_named unimplemented for non-typedef - {:#?}",
            declarator
        )
    }
}

fn has_parameters(parameter_type_list: &ParameterTypeList) -> bool {
    let declarations = &parameter_type_list.parameter_declarations;

    if !declarations.is_empty() {
        if let Some(first) = declarations.first() {
            if first.core.is_nothing()
                && first.attributes.is_empty()
                && first.declaration_specifiers.attributes.is_empty()
                && first
                    .declaration_specifiers
                    .specifiers
                    .first()
                    .map_or(false, |first| first.kind.is_void())
                && first.declaration_specifiers.specifiers.len() == 1
            {
                // This function has parameters `(void)`
                return false;
            }
        }
    }

    // Technically, an empty parameter list means to accept any number of arguments, e.g.
    // when not `(void)`, but we don't support that yet, so just assume zero when that occurs
    parameter_type_list.parameter_declarations.len() != 0
}

pub fn declare_function(
    typedefs: &mut HashMap<String, CTypedef>,
    ast_file: &mut File,
    _attribute_specifiers: &[()],
    declaration_specifiers: &DeclarationSpecifiers,
    declarator: &Declarator,
    parameter_type_list: &ParameterTypeList,
) -> Result<(), ParseError> {
    let source = declarator.source;
    let (name, return_type, is_typedef) = get_name_and_type(
        ast_file,
        typedefs,
        declarator,
        declaration_specifiers,
        false,
    )?;
    let mut required = vec![];

    if has_parameters(&parameter_type_list) {
        for param in parameter_type_list.parameter_declarations.iter() {
            let (name, ast_type, is_typedef) = match &param.core {
                ParameterDeclarationCore::Declarator(declarator) => get_name_and_type(
                    ast_file,
                    typedefs,
                    declarator,
                    &param.declaration_specifiers,
                    true,
                )?,
                ParameterDeclarationCore::AbstractDeclarator(_) => todo!(),
                ParameterDeclarationCore::Nothing => todo!(),
            };

            if is_typedef {
                return Err(
                    ParseErrorKind::Misc("Parameter type cannot be typedef").at(param.source)
                );
            }

            required.push(Parameter { name, ast_type });
        }
    }

    if is_typedef {
        let ast_type = ast::TypeKind::FunctionPointer(ast::FunctionPointer {
            parameters: required,
            return_type: Box::new(return_type),
            is_cstyle_variadic: parameter_type_list.is_variadic,
        })
        .at(declarator.source);

        typedefs.insert(name, CTypedef { ast_type });
        return Ok(());
    }

    let parameters = Parameters {
        required,
        is_cstyle_vararg: parameter_type_list.is_variadic,
    };

    ast_file.functions.push(Function {
        name,
        parameters,
        return_type,
        stmts: vec![],
        is_foreign: true,
        source,
    });

    Ok(())
}
