use super::{
    error::ParseErrorKind, DeclarationSpecifierKind, DeclarationSpecifiers,
    Declarator, DeclaratorKind, ParameterTypeList, ParseError, Pointers, TypeQualifier,
    TypeSpecifier, TypeSpecifierKind, TypeSpecifierQualifier,
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
    let source = declarator.source;
    let (name, return_type) = get_name_and_type(declarator, declaration_specifiers)?;
    let mut required = vec![];

    for param in parameter_type_list.parameter_declarations.iter() {
        let (name, ast_type) = match &param.core {
            ParameterDeclarationCore::Declarator(declarator) => {
                get_name_and_type(declarator, &param.declaration_specifiers)?
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
        source,
    });

    Ok(())
}

fn get_name_and_type(
    declarator: &Declarator,
    declaration_specifiers: &DeclarationSpecifiers,
) -> Result<(String, Type), ParseError> {
    let (name, pointers) = get_name_and_pointers(declarator)?;
    let mut ast_type = get_base_type(declaration_specifiers, declarator.source)?;

    for pointer in pointers.pointers.iter() {
        ast_type = Type::new(TypeKind::Pointer(Box::new(ast_type)), pointer.source);

        for type_qualfier in pointer.type_qualifiers.iter() {
            ast_type = augment_ast_type_with_type_qualifier(Some(ast_type), type_qualfier)?
                .expect("type is present");
        }
    }

    Ok((name, ast_type))
}

fn get_name_and_pointers(declarator: &Declarator) -> Result<(String, Pointers), ParseError> {
    match &declarator.kind {
        DeclaratorKind::Named(name) => Ok((name.to_string(), Pointers::default())),
        DeclaratorKind::Pointers(inner, pointers) => {
            let (name, more_pointers) = get_name_and_pointers(inner)?;
            Ok((name, pointers.concat(&more_pointers)))
        }
        DeclaratorKind::Function(_, _) => Err(ParseError::new(
            ParseErrorKind::CannotReturnFunctionPointerType,
            declarator.source,
        )),
    }
}

fn get_base_type(
    declaration_specifiers: &DeclarationSpecifiers,
    parent_source: Source,
) -> Result<Type, ParseError> {
    let mut ast_type = None;

    for specifier in declaration_specifiers.specifiers.iter() {
        match &specifier.kind {
            DeclarationSpecifierKind::Auto => {
                return Err(ParseError::new(
                    ParseErrorKind::AutoNotSupportedForReturnType,
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::Constexpr => {
                return Err(ParseError::new(
                    ParseErrorKind::ConstexprNotSupportedForReturnType,
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::Extern => todo!(),
            DeclarationSpecifierKind::Register => todo!(),
            DeclarationSpecifierKind::Static => todo!(),
            DeclarationSpecifierKind::ThreadLocal => todo!(),
            DeclarationSpecifierKind::Typedef => todo!(),
            DeclarationSpecifierKind::Inline => todo!(),
            DeclarationSpecifierKind::Noreturn => todo!(),
            DeclarationSpecifierKind::TypeSpecifierQualifier(type_specifier_qualifier) => {
                ast_type = augment_ast_type(ast_type, type_specifier_qualifier)?;
            }
        }
    }

    match ast_type {
        Some(ast_type) => Ok(ast_type),
        None => Err(ParseError::new(
            ParseErrorKind::Misc("Missing base type"),
            parent_source,
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
            Some(..) => Err(ParseError::new(
                ParseErrorKind::InvalidType,
                type_specifier.source,
            )),
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
        Some(..) => Err(ParseError::new(ParseErrorKind::InvalidType, source)),
        None => Ok(Some(Type::new(TypeKind::Integer { bits, sign }, source))),
    }
}
