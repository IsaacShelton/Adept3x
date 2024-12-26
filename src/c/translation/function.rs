use super::{parameters::has_parameters, types::get_name_and_type};
use crate::{
    ast::{self, AstFile, Function, FunctionHead, Parameter, Parameters, Privacy},
    c::parser::{
        error::ParseErrorKind, CTypedef, DeclarationSpecifiers, Declarator,
        ParameterDeclarationCore, ParameterTypeList, ParseError,
    },
    diagnostics::Diagnostics,
};
use std::collections::HashMap;

pub fn declare_function(
    typedefs: &mut HashMap<String, CTypedef>,
    ast_file: &mut AstFile,
    _attribute_specifiers: &[()],
    declaration_specifiers: &DeclarationSpecifiers,
    declarator: &Declarator,
    parameter_type_list: &ParameterTypeList,
    diagnostics: &Diagnostics,
) -> Result<(), ParseError> {
    let source = declarator.source;
    let (name, return_type, is_typedef) = get_name_and_type(
        ast_file,
        typedefs,
        declarator,
        declaration_specifiers,
        false,
        diagnostics,
    )?;
    let mut required = vec![];

    if has_parameters(parameter_type_list) {
        for param in parameter_type_list.parameter_declarations.iter() {
            let (name, ast_type, is_typedef) = match &param.core {
                ParameterDeclarationCore::Declarator(declarator) => get_name_and_type(
                    ast_file,
                    typedefs,
                    declarator,
                    &param.declaration_specifiers,
                    true,
                    diagnostics,
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

    let head = FunctionHead {
        name,
        givens: vec![],
        parameters,
        return_type,
        is_foreign: true,
        source,
        abide_abi: true,
        tag: None,
        privacy: Privacy::Public,
    };

    ast_file.functions.push(Function {
        head,
        stmts: vec![],
    });

    Ok(())
}
