use super::{parameters::has_parameters, types::get_name_and_type};
use crate::{
    asg::TypeParams,
    ast::{self, AstFile, Func, FuncHead, Param, Params, Privacy},
    c::parser::{
        error::ParseErrorKind, CTypedef, DeclarationSpecifiers, Declarator,
        ParameterDeclarationCore, ParameterTypeList, ParseError, StorageClassSpecifier,
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
    let (name, return_type, storage_class, function_specifier, _is_thread_local) =
        get_name_and_type(
            ast_file,
            typedefs,
            declarator,
            declaration_specifiers,
            false,
            diagnostics,
        )?;
    let mut required = vec![];

    if function_specifier.is_some() {
        return Err(ParseErrorKind::Misc("Function specifiers are not supported yet").at(source));
    }

    if has_parameters(parameter_type_list) {
        for param in parameter_type_list.parameter_declarations.iter() {
            let (name, ast_type, storage_class, function_specifier, _) = match &param.core {
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

            if storage_class.is_some() {
                return Err(
                    ParseErrorKind::Misc("Storage classes not support on typedef").at(param.source),
                );
            }

            if function_specifier.is_some() {
                return Err(
                    ParseErrorKind::Misc("Function specifiers cannot be used on typedef")
                        .at(source),
                );
            }

            required.push(Param { name, ast_type });
        }
    }

    match storage_class {
        Some(StorageClassSpecifier::Typedef) => {
            let ast_type = ast::TypeKind::FuncPtr(ast::FuncPtr {
                parameters: required,
                return_type: Box::new(return_type),
                is_cstyle_variadic: parameter_type_list.is_variadic,
            })
            .at(declarator.source);

            typedefs.insert(name, CTypedef { ast_type });
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
        name,
        type_params: TypeParams::default(),
        givens: vec![],
        params: Params {
            required,
            is_cstyle_vararg: parameter_type_list.is_variadic,
        },
        return_type,
        is_foreign: true,
        source,
        abide_abi: true,
        tag: None,
        privacy: Privacy::Public,
    };

    ast_file.funcs.push(Func {
        head,
        stmts: vec![],
    });

    Ok(())
}
