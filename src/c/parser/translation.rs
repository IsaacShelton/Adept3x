use super::{
    error::ParseErrorKind, DeclarationSpecifier, DeclarationSpecifiers, Declarator, ParameterTypeList, ParseError, Pointers, TypeSpecifier, TypeSpecifierKind, TypeSpecifierQualifier
};
use crate::ast::{File, Function, Type, TypeKind};

pub fn declare_function(
    ast_file: &mut File,
    _attribute_specifiers: &[()],
    declaration_specifiers: &DeclarationSpecifiers,
    declarator: &Declarator,
    _parameter_type_list: &ParameterTypeList,
) -> Result<(), ParseError> {
    let (name, pointers) = get_name_and_pointers(declarator)?;

    let return_type = get_return_type(declaration_specifiers);

    println!("name is {}", name);

    ast_file.functions.push(Function {
        name,
        parameters: todo!(),
        return_type: todo!(),
        stmts: todo!(),
        is_foreign: todo!(),
    });
}

fn get_name_and_pointers(declarator: &Declarator) -> Result<(String, Pointers), ParseError> {
    match declarator {
        Declarator::Named(name) => Ok((name.to_string(), Pointers::default())),
        Declarator::Pointers(inner, pointers) => {
            let (name, more_pointers) = get_name_and_pointers(inner)?;
            Ok((name, pointers.concat(&more_pointers)))
        }
        Declarator::Function(..) => Err(ParseError::new(
            ParseErrorKind::CannotReturnFunctionPointerType,
            None,
        )),
    }
}

fn get_return_type(declaration_specifiers: &DeclarationSpecifiers) -> Result<Type, ParseError> {
    let mut ast_type = None;

    for specifier in declaration_specifiers.specifiers.iter() {
        match specifier {
            DeclarationSpecifier::Auto => {
                return Err(ParseError::new(
                    ParseErrorKind::AutoNotSupportedForReturnType,
                    None,
                ))
            }
            DeclarationSpecifier::Constexpr => {
                return Err(ParseError::new(
                    ParseErrorKind::ConstexprNotSupportedForReturnType,
                    None,
                ))
            }
            DeclarationSpecifier::Extern => todo!(),
            DeclarationSpecifier::Register => todo!(),
            DeclarationSpecifier::Static => todo!(),
            DeclarationSpecifier::ThreadLocal => todo!(),
            DeclarationSpecifier::Typedef => todo!(),
            DeclarationSpecifier::Inline => todo!(),
            DeclarationSpecifier::Noreturn => todo!(),
            DeclarationSpecifier::TypeSpecifierQualifier(type_specifier_qualifier) => {
                ast_type = augment_ast_type(ast_type, type_specifier_qualifier)?;
            }
        }
    }

    match ast_type {
        Some(ast_type) => Ok(ast_type),
        None => Err(ParseError::new(
            ParseErrorKind::Misc("Function is missing return type"),
            None,
        )),
    }
}

fn augment_ast_type(
    ast_type: Option<Type>,
    type_specifier_qualifier: &TypeSpecifierQualifier,
) -> Result<Option<Type>, ParseError> {
    match type_specifier_qualifier {
        TypeSpecifierQualifier::TypeSpecifier(type_specifier) => {
            augment_ast_type_with_type_specifier(ast_type, type_specifier)
        }
        TypeSpecifierQualifier::TypeQualifier(_) => todo!(),
        TypeSpecifierQualifier::AlignmentSpecifier(_) => todo!(),
    }
}

fn augment_ast_type_with_type_specifier(
    ast_type: Option<Type>,
    type_specifier: &TypeSpecifier,
) -> Result<Option<Type>, ParseError> {
    match type_specifier.kind {
        TypeSpecifierKind::Void => match ast_type {
            Some(..) => Err(ParseError::new(ParseErrorKind::InvalidType, None)),
            None => Ok(Some(Type::new(TypeKind::Void, type_specifier.source))),
        },
        TypeSpecifierKind::Char => todo!(),
        TypeSpecifierKind::Short => todo!(),
        TypeSpecifierKind::Int => todo!(),
        TypeSpecifierKind::Long => todo!(),
        TypeSpecifierKind::Float => todo!(),
        TypeSpecifierKind::Double => todo!(),
        TypeSpecifierKind::Signed => todo!(),
        TypeSpecifierKind::Unsigned => todo!(),
    }
}
