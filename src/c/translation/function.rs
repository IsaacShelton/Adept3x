use super::{parameters::has_parameters, types::get_name_and_type};
use crate::{
    asg::TypeParams,
    ast::{self, AstFile, Func, FuncHead, Param, Params},
    c::{
        ast::{
            Attribute, CTypedef, DeclarationSpecifiers, Declarator, ParameterDeclarationCore,
            ParameterTypeList, StorageClassSpecifier,
        },
        parser::{error::ParseErrorKind, ParseError},
    },
    diagnostics::Diagnostics,
    workspace::compile::c_code::CFileType,
};
use std::collections::HashMap;

pub fn declare_function(
    typedefs: &mut HashMap<String, CTypedef>,
    ast_file: &mut AstFile,
    _attribute_specifiers: &[Attribute],
    declaration_specifiers: &DeclarationSpecifiers,
    declarator: &Declarator,
    parameter_type_list: &ParameterTypeList,
    diagnostics: &Diagnostics,
    c_file_type: CFileType,
) -> Result<(), ParseError> {
    let source = declarator.source;
    let func_info = get_name_and_type(
        ast_file,
        typedefs,
        declarator,
        declaration_specifiers,
        false,
        diagnostics,
    )?;
    let mut required = vec![];

    if func_info.specifiers.function_specifier.is_some() {
        return Err(ParseErrorKind::Misc("Function specifiers are not supported yet").at(source));
    }

    if has_parameters(parameter_type_list) {
        for param in parameter_type_list.parameter_declarations.iter() {
            let param_info = match &param.core {
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

            if param_info.specifiers.storage_class.is_some() {
                return Err(
                    ParseErrorKind::Misc("Storage classes not support on typedef").at(param.source),
                );
            }

            if param_info.specifiers.function_specifier.is_some() {
                return Err(
                    ParseErrorKind::Misc("Function specifiers cannot be used on typedef")
                        .at(source),
                );
            }

            required.push(Param::new(Some(param_info.name), param_info.ast_type));
        }
    }

    match func_info.specifiers.storage_class {
        Some(StorageClassSpecifier::Typedef) => {
            let ast_type = ast::TypeKind::FuncPtr(ast::FuncPtr {
                parameters: required,
                return_type: Box::new(func_info.ast_type),
                is_cstyle_variadic: parameter_type_list.is_variadic,
            })
            .at(declarator.source);

            typedefs.insert(func_info.name, CTypedef { ast_type });
            return Ok(());
        }
        Some(_) => {
            return Err(
                ParseErrorKind::Misc("Unsupported storage class here").at(declarator.source)
            );
        }
        None => (),
    }

    let head = FuncHead {
        name: func_info.name,
        type_params: TypeParams::default(),
        givens: vec![],
        params: Params {
            required,
            is_cstyle_vararg: parameter_type_list.is_variadic,
        },
        return_type: func_info.ast_type,
        is_foreign: true,
        source,
        abide_abi: true,
        tag: None,
        privacy: c_file_type.privacy(),
    };

    ast_file.funcs.push(Func {
        head,
        stmts: vec![],
    });

    Ok(())
}
