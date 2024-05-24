use super::{
    error::ParseErrorKind, DeclarationSpecifier, DeclarationSpecifiers, Declarator,
    ParameterTypeList, ParseError, Pointers, TypeQualifier, TypeSpecifier, TypeSpecifierKind,
    TypeSpecifierQualifier,
};
use crate::{
    ast::{
        File, Function, IntegerBits, IntegerSign, Parameter, Parameters, Source, Type, TypeKind,
    },
    c::parser::ParameterDeclarationCore,
};

pub fn declare_function(
    ast_file: &mut File,
    _attribute_specifiers: &[()],
    declaration_specifiers: &DeclarationSpecifiers,
    declarator: &Declarator,
    parameter_type_list: &ParameterTypeList,
) -> Result<(), ParseError> {
    let (name, return_type) = get_function_name_and_type(declarator, declaration_specifiers)?;
    let mut required = vec![];

    for param in parameter_type_list.parameter_declarations.iter() {
        let (name, ast_type) = match &param.core {
            ParameterDeclarationCore::Declarator(declarator) => {
                get_function_name_and_type(declarator, &param.declaration_specifiers)?
            }
            ParameterDeclarationCore::AbstractDeclarator(_) => todo!(),
            ParameterDeclarationCore::Nothing => todo!(),
        };

        required.push(Parameter { name, ast_type });
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
    });

    Ok(())
}

fn get_function_name_and_type(
    declarator: &Declarator,
    declaration_specifiers: &DeclarationSpecifiers,
) -> Result<(String, Type), ParseError> {
    let (name, pointers) = get_name_and_pointers(declarator)?;
    let mut ast_type = get_return_type(declaration_specifiers)?;

    for pointer in pointers.pointers.iter() {
        for qualfier in pointer.type_qualifiers.iter() {
            match qualfier {
                TypeQualifier::Const => (),
                TypeQualifier::Restrict => todo!(),
                TypeQualifier::Volatile => todo!(),
                TypeQualifier::Atomic => todo!(),
            }
        }

        ast_type = Type::new(TypeKind::Pointer(Box::new(ast_type)), pointer.source);
    }

    Ok((name, ast_type))
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
        TypeSpecifierQualifier::TypeQualifier(type_qualifier) => {
            augment_ast_type_with_type_qualifier(ast_type, type_qualifier)
        }
        TypeSpecifierQualifier::AlignmentSpecifier(_) => todo!(),
    }
}

fn augment_ast_type_with_type_qualifier(
    ast_type: Option<Type>,
    type_qualifier: &TypeQualifier,
) -> Result<Option<Type>, ParseError> {
    match type_qualifier {
        TypeQualifier::Const => Ok(ast_type),
        TypeQualifier::Restrict => todo!(),
        TypeQualifier::Volatile => todo!(),
        TypeQualifier::Atomic => todo!(),
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
        TypeSpecifierKind::Char => augment_ast_type_as_integer(
            ast_type,
            IntegerBits::Bits8,
            IntegerSign::Unsigned,
            type_specifier.source,
        ),
        TypeSpecifierKind::Short => todo!(),
        TypeSpecifierKind::Int => augment_ast_type_as_integer(
            ast_type,
            IntegerBits::Bits8,
            IntegerSign::Signed,
            type_specifier.source,
        ),
        TypeSpecifierKind::Long => todo!(),
        TypeSpecifierKind::Float => todo!(),
        TypeSpecifierKind::Double => todo!(),
        TypeSpecifierKind::Signed => todo!(),
        TypeSpecifierKind::Unsigned => todo!(),
    }
}

fn augment_ast_type_as_integer(
    ast_type: Option<Type>,
    bits: IntegerBits,
    sign: IntegerSign,
    source: Source,
) -> Result<Option<Type>, ParseError> {
    match ast_type {
        Some(..) => Err(ParseError::new(ParseErrorKind::InvalidType, None)),
        None => Ok(Some(Type::new(TypeKind::Integer { bits, sign }, source))),
    }
}
