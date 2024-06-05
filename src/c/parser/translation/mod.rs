mod type_base;

use super::{
    error::ParseErrorKind, CTypedef, DeclarationSpecifiers, Declarator, ParameterTypeList,
    ParseError,
};
use crate::{
    ast::{File, Function, Parameter, Parameters},
    c::parser::{translation::type_base::get_name_and_type, ParameterDeclarationCore},
};
use std::collections::HashMap;

pub fn declare_named(
    _ast_file: &mut File,
    declarator: &Declarator,
    _attribute_specifiers: &[()],
    declaration_specifiers: &DeclarationSpecifiers,
    name: &str,
    typedefs: &mut HashMap<String, CTypedef>,
) -> Result<(), ParseError> {
    println!("{} {:#?}", name, declaration_specifiers);

    let (name, ast_type, is_typedef) =
        get_name_and_type(typedefs, declarator, declaration_specifiers, false)?;

    if is_typedef {
        typedefs.insert(name.to_string(), CTypedef { ast_type });
        Ok(())
    } else {
        todo!()
    }
}

pub fn declare_function(
    typedefs: &HashMap<String, CTypedef>,
    ast_file: &mut File,
    _attribute_specifiers: &[()],
    declaration_specifiers: &DeclarationSpecifiers,
    declarator: &Declarator,
    parameter_type_list: &ParameterTypeList,
) -> Result<(), ParseError> {
    let source = declarator.source;
    let (name, return_type, is_typedef) =
        get_name_and_type(typedefs, declarator, declaration_specifiers, false)?;
    let mut required = vec![];

    for param in parameter_type_list.parameter_declarations.iter() {
        let (name, ast_type, is_typedef) = match &param.core {
            ParameterDeclarationCore::Declarator(declarator) => {
                get_name_and_type(typedefs, declarator, &param.declaration_specifiers, true)?
            }
            ParameterDeclarationCore::AbstractDeclarator(_) => todo!(),
            ParameterDeclarationCore::Nothing => todo!(),
        };

        if is_typedef {
            return Err(ParseErrorKind::Misc("Parameter type cannot be typedef").at(param.source));
        }

        required.push(Parameter { name, ast_type });
    }

    if is_typedef {
        todo!();
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
