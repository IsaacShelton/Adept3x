use std::collections::HashMap;

use super::{
    error::ParseErrorKind, CTypedef, CompositeKind, DeclarationSpecifierKind, DeclarationSpecifiers, Declarator, DeclaratorKind, ParameterTypeList, ParseError, Pointers, TypeQualifier, TypeSpecifier, TypeSpecifierKind, TypeSpecifierQualifier
};
use crate::{
    ast::{
        File, Function, IntegerBits, IntegerSign, Parameter, Parameters, Source, Type, TypeKind,
    },
    c::parser::ParameterDeclarationCore,
};

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
        get_name_and_type(typedefs, declarator, declaration_specifiers)?;

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
        get_name_and_type(typedefs, declarator, declaration_specifiers)?;
    let mut required = vec![];

    for param in parameter_type_list.parameter_declarations.iter() {
        let (name, ast_type, is_typedef) = match &param.core {
            ParameterDeclarationCore::Declarator(declarator) => {
                get_name_and_type(typedefs, declarator, &param.declaration_specifiers)?
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

fn get_name_and_type(
    typedefs: &HashMap<String, CTypedef>,
    declarator: &Declarator,
    declaration_specifiers: &DeclarationSpecifiers,
) -> Result<(String, Type, bool), ParseError> {
    let (name, pointers) = get_name_and_decorators(declarator)?;
    let (mut ast_type, is_typedef) =
        get_base_type(typedefs, declaration_specifiers, declarator.source)?;

    for pointer in pointers.pointers.iter() {
        ast_type = Type::new(TypeKind::Pointer(Box::new(ast_type)), pointer.source);

        for type_qualfier in pointer.type_qualifiers.iter() {
            ast_type = augment_ast_type_with_type_qualifier(Some(ast_type), type_qualfier)?
                .expect("type is present");
        }
    }

    Ok((name, ast_type, is_typedef))
}

fn get_name_and_decorators(declarator: &Declarator) -> Result<(String, Pointers), ParseError> {
    match &declarator.kind {
        DeclaratorKind::Named(name) => Ok((name.to_string(), Pointers::default())),
        DeclaratorKind::Pointers(inner, pointers) => {
            let (name, more_pointers) = get_name_and_decorators(inner)?;
            Ok((name, pointers.concat(&more_pointers)))
        }
        DeclaratorKind::Function(..) => Err(ParseError::new(
            ParseErrorKind::CannotReturnFunctionPointerType,
            declarator.source,
        )),
        DeclaratorKind::Array(..) => {
            todo!()
        }
    }
}

fn get_base_type(
    typedefs: &HashMap<String, CTypedef>,
    declaration_specifiers: &DeclarationSpecifiers,
    parent_source: Source,
) -> Result<(Type, bool), ParseError> {
    let mut ast_type = None;
    let mut is_typedef = false;

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
            DeclarationSpecifierKind::Typedef => is_typedef = true,
            DeclarationSpecifierKind::Inline => todo!(),
            DeclarationSpecifierKind::Noreturn => todo!(),
            DeclarationSpecifierKind::TypeSpecifierQualifier(type_specifier_qualifier) => {
                ast_type = augment_ast_type(typedefs, ast_type, type_specifier_qualifier)?;
            }
        }
    }

    match ast_type {
        Some(ast_type) => Ok((ast_type, is_typedef)),
        None => Err(ParseError::new(
            ParseErrorKind::Misc("Missing base type"),
            parent_source,
        )),
    }
}

fn augment_ast_type(
    typedefs: &HashMap<String, CTypedef>,
    ast_type: Option<Type>,
    type_specifier_qualifier: &TypeSpecifierQualifier,
) -> Result<Option<Type>, ParseError> {
    match type_specifier_qualifier {
        TypeSpecifierQualifier::TypeSpecifier(type_specifier) => {
            augment_ast_type_with_type_specifier(typedefs, ast_type, type_specifier)
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
    typedefs: &HashMap<String, CTypedef>,
    ast_type: Option<Type>,
    type_specifier: &TypeSpecifier,
) -> Result<Option<Type>, ParseError> {
    match &type_specifier.kind {
        TypeSpecifierKind::Void => match ast_type {
            Some(..) => Err(ParseError::new(
                ParseErrorKind::InvalidType,
                type_specifier.source,
            )),
            None => Ok(Some(Type::new(TypeKind::Void, type_specifier.source))),
        },
        TypeSpecifierKind::Bool => augment_ast_type_as_integer(
            ast_type,
            IntegerBits::Bits8,
            IntegerSign::Unsigned,
            type_specifier.source,
        ),
        TypeSpecifierKind::Char => augment_ast_type_as_integer(
            ast_type,
            IntegerBits::Bits8,
            IntegerSign::Unsigned,
            type_specifier.source,
        ),
        TypeSpecifierKind::Short => augment_ast_type_as_integer(
            ast_type,
            IntegerBits::Bits16,
            IntegerSign::Signed,
            type_specifier.source,
        ),
        TypeSpecifierKind::Int => augment_ast_type_as_integer(
            ast_type,
            IntegerBits::Bits8,
            IntegerSign::Signed,
            type_specifier.source,
        ),
        TypeSpecifierKind::Long => augment_ast_type_as_integer(
            ast_type,
            IntegerBits::Bits64,
            IntegerSign::Signed,
            type_specifier.source,
        ),
        TypeSpecifierKind::Float => todo!(),
        TypeSpecifierKind::Double => todo!(),
        TypeSpecifierKind::Signed => todo!(),
        TypeSpecifierKind::Unsigned => todo!(),
        TypeSpecifierKind::Composite(composite) => {
            if !composite.attributes.is_empty() {
                return Err(
                    ParseErrorKind::Misc("Attributes not supported on composites")
                        .at(composite.source),
                );
            }

            if let Some(members) = &composite.members {
                match composite.kind {
                    CompositeKind::Struct => {
                        todo!("struct composites")
                        // Ok(TypeKind::AnonymousStruct().pod().at(composite.source))
                    },
                    CompositeKind::Union => {
                        todo!("union composites")
                        // Ok(TypeKind::AnonymousUnion().pod().at(composite.source))
                    }
                }
            } else {
                todo!("unfinished composites");
            }
        }
        TypeSpecifierKind::Enumeration(_enumeration) => todo!(),
        TypeSpecifierKind::TypedefName(typedef_name) => {
            let new_ast_type = typedefs
                .get(&typedef_name.name)
                .expect("typedef exists")
                .ast_type
                .clone();

            match ast_type {
                Some(..) => Err(ParseError::new(
                    ParseErrorKind::InvalidType,
                    type_specifier.source,
                )),
                None => Ok(Some(Type::new(new_ast_type.kind, type_specifier.source))),
            }
        }
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
