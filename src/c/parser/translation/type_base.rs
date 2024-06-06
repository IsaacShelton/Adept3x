use crate::{
    ast::{
        AnonymousEnum, AnonymousStruct, EnumMember, Field, FixedArray, FloatSize, FunctionPointer,
        IntegerBits, IntegerSign, Parameter, Privacy, Source, Type, TypeKind,
    },
    c::parser::{
        error::ParseErrorKind, translation::eval::evaluate_to_const_integer,
        AlignmentSpecifierKind, ArrayQualifier, CTypedef, Composite, CompositeKind,
        DeclarationSpecifierKind, DeclarationSpecifiers, Declarator, DeclaratorKind, Decorator,
        Decorators, Enumeration, FunctionQualifier, MemberDeclaration, MemberDeclarator,
        ParameterDeclarationCore, ParseError, Pointer, TypeQualifierKind, TypeSpecifierKind,
        TypeSpecifierQualifier,
    },
};
use indexmap::IndexMap;
use num_bigint::BigInt;
use num_traits::Zero;
use std::collections::HashMap;

use super::expr::translate_expr;

#[derive(Debug)]
pub struct TypeBase {
    pub ast_type: Type,
    pub is_typedef: bool,
}

#[derive(Debug)]
pub struct TypeBaseBuilder {
    pub source: Source,
    pub bits: Option<IntegerBits>,
    pub sign: Option<IntegerSign>,
    pub concrete: Option<Type>,
    pub is_typedef: bool,
}

impl TypeBaseBuilder {
    pub fn new(source: Source) -> Self {
        Self {
            source,
            bits: None,
            sign: None,
            concrete: None,
            is_typedef: false,
        }
    }

    pub fn build(self) -> Result<TypeBase, ParseError> {
        let ast_type = if let Some(concrete) = self.concrete {
            concrete
        } else if let Some(bits) = self.bits {
            let sign = if let Some(sign) = self.sign {
                sign
            } else if bits == IntegerBits::Bits8 {
                IntegerSign::Unsigned
            } else {
                IntegerSign::Signed
            };

            Type::new(TypeKind::Integer { bits, sign }, self.source)
        } else if let Some(sign) = self.sign {
            Type::new(
                TypeKind::Integer {
                    bits: IntegerBits::Bits32,
                    sign,
                },
                self.source,
            )
        } else {
            return Err(ParseErrorKind::InvalidType.at(self.source));
        };

        let is_typedef = self.is_typedef;

        Ok(TypeBase {
            ast_type,
            is_typedef,
        })
    }

    pub fn void(&mut self, source: Source) -> Result<(), ParseError> {
        self.concrete(TypeKind::Void, source)
    }

    pub fn concrete(&mut self, type_kind: TypeKind, source: Source) -> Result<(), ParseError> {
        if self.bits.is_some() || self.sign.is_some() || self.concrete.is_some() {
            return Err(ParseErrorKind::InvalidType.at(source));
        }

        self.concrete = Some(Type::new(type_kind, source));
        Ok(())
    }

    pub fn bool(&mut self, source: Source) -> Result<(), ParseError> {
        self.concrete(TypeKind::Boolean, source)
    }

    pub fn integer(&mut self, bits: IntegerBits, source: Source) -> Result<(), ParseError> {
        if self.bits.is_some() || self.concrete.is_some() {
            return Err(ParseErrorKind::InvalidType.at(source));
        }

        self.bits = Some(bits);
        Ok(())
    }

    pub fn long(&mut self, source: Source) -> Result<(), ParseError> {
        if self.concrete.is_some() {
            return Err(ParseErrorKind::InvalidType.at(source));
        }

        self.bits = Some(IntegerBits::Bits64);
        Ok(())
    }

    pub fn float(&mut self, source: Source) -> Result<(), ParseError> {
        self.concrete(TypeKind::Float(FloatSize::Bits32), source)
    }

    pub fn double(&mut self, source: Source) -> Result<(), ParseError> {
        self.concrete(TypeKind::Float(FloatSize::Bits64), source)
    }

    pub fn sign(&mut self, sign: IntegerSign, source: Source) -> Result<(), ParseError> {
        if self.sign.is_some() || self.concrete.is_some() {
            return Err(ParseErrorKind::InvalidType.at(source));
        }

        self.sign = Some(sign);
        Ok(())
    }

    pub fn constant(&mut self) -> Result<(), ParseError> {
        // NOTE: We are ignoring `const` for now
        Ok(())
    }
}

pub fn get_type_base(
    typedefs: &HashMap<String, CTypedef>,
    declaration_specifiers: &DeclarationSpecifiers,
    parent_source: Source,
) -> Result<TypeBase, ParseError> {
    let mut builder = TypeBaseBuilder::new(parent_source);

    if !declaration_specifiers.attributes.is_empty() {
        return Err(ParseError::message(
            "Attributes on declaration specifiers not supported yet",
            parent_source,
        ));
    }

    for specifier in declaration_specifiers.specifiers.iter() {
        match &specifier.kind {
            DeclarationSpecifierKind::Auto => {
                return Err(ParseError::message(
                    "'auto' not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::Constexpr => {
                return Err(ParseError::message(
                    "'constexpr' not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::Extern => {
                return Err(ParseError::message(
                    "'extern' not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::Register => {
                return Err(ParseError::message(
                    "'register' declaration specifier not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::Static => {
                return Err(ParseError::message(
                    "'static' declaration specifier not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::ThreadLocal => {
                return Err(ParseError::message(
                    "'thread_local' declaration specifier not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::Typedef => builder.is_typedef = true,
            DeclarationSpecifierKind::Inline => {
                return Err(ParseError::message(
                    "'inline' declaration specifier not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::Noreturn => {
                return Err(ParseError::message(
                    "'_Noreturn' declaration specifier not supported yet",
                    specifier.source,
                ))
            }
            DeclarationSpecifierKind::TypeSpecifierQualifier(tsq) => match tsq {
                TypeSpecifierQualifier::TypeSpecifier(ts) => match &ts.kind {
                    TypeSpecifierKind::Void => builder.void(ts.source)?,
                    TypeSpecifierKind::Bool => builder.bool(ts.source)?,
                    TypeSpecifierKind::Char => builder.integer(IntegerBits::Bits8, ts.source)?,
                    TypeSpecifierKind::Short => builder.integer(IntegerBits::Bits16, ts.source)?,
                    TypeSpecifierKind::Int => builder.integer(IntegerBits::Bits32, ts.source)?,
                    TypeSpecifierKind::Long => builder.long(ts.source)?,
                    TypeSpecifierKind::Float => builder.float(ts.source)?,
                    TypeSpecifierKind::Double => builder.double(ts.source)?,
                    TypeSpecifierKind::Signed => builder.sign(IntegerSign::Signed, ts.source)?,
                    TypeSpecifierKind::Unsigned => {
                        builder.sign(IntegerSign::Unsigned, ts.source)?
                    }
                    TypeSpecifierKind::Composite(composite) => builder
                        .concrete(make_anonymous_composite(typedefs, composite)?, ts.source)?,
                    TypeSpecifierKind::Enumeration(enumeration) => {
                        builder.concrete(make_anonymous_enum(enumeration)?, ts.source)?
                    }
                    TypeSpecifierKind::TypedefName(typedef_name) => {
                        let ast_type = typedefs
                            .get(&typedef_name.name)
                            .expect("typedef exists")
                            .ast_type
                            .clone();

                        builder.concrete(ast_type.kind, typedef_name.source)?
                    }
                },
                TypeSpecifierQualifier::TypeQualifier(tq) => match &tq.kind {
                    TypeQualifierKind::Const => builder.constant()?,
                    TypeQualifierKind::Restrict => todo!(),
                    TypeQualifierKind::Volatile => todo!(),
                    TypeQualifierKind::Atomic => todo!(),
                },
                TypeSpecifierQualifier::AlignmentSpecifier(al) => match &al.kind {
                    AlignmentSpecifierKind::AlignAsType(_) => todo!(),
                    AlignmentSpecifierKind::AlisnAsConstExpr(_) => todo!(),
                },
            },
        }
    }

    builder.build()
}

pub fn make_anonymous_enum(enumeration: &Enumeration) -> Result<TypeKind, ParseError> {
    match enumeration {
        Enumeration::Definition(definition) => {
            if !definition.attributes.is_empty() {
                todo!("enum attributes not supported yet")
            }

            let mut members = IndexMap::with_capacity(definition.body.len());
            let mut next_value = BigInt::zero();

            for enumerator in definition.body.iter() {
                if !enumerator.attributes.is_empty() {
                    todo!("attributes not supported on enum members yet");
                }

                let value = if let Some(value) = &enumerator.value {
                    evaluate_to_const_integer(&value.value)?
                } else {
                    let value = next_value.clone();
                    next_value += 1;
                    value
                };

                let enum_member = EnumMember {
                    value,
                    explicit_value: enumerator.value.is_some(),
                };

                if members
                    .insert(enumerator.name.clone(), enum_member)
                    .is_some()
                {
                    return Err(ParseErrorKind::DuplicateEnumMember(enumerator.name.clone())
                        .at(enumerator.source));
                }
            }

            if definition.enum_type_specifier.is_some() {
                todo!("anonymous enum type specifiers not supported yet");
            }

            Ok(TypeKind::AnonymousEnum(AnonymousEnum { members }))
        }
        Enumeration::Reference(reference) => {
            if reference.enum_type_specifier.is_some() {
                todo!("support enum type specifiers")
            }

            Ok(TypeKind::EnumNamed(reference.name.clone()))
        }
    }
}

pub fn make_anonymous_composite(
    typedefs: &HashMap<String, CTypedef>,
    composite: &Composite,
) -> Result<TypeKind, ParseError> {
    if !composite.attributes.is_empty() {
        return Err(
            ParseErrorKind::Misc("attributes not supported on composites yet").at(composite.source),
        );
    }

    let members = if let Some(members) = composite.members.as_ref() {
        members
    } else {
        let name = composite.name.as_ref().ok_or_else(|| {
            ParseErrorKind::Misc("incomplete struct must have name").at(composite.source)
        })?;

        return Ok(match &composite.kind {
            CompositeKind::Struct => TypeKind::StructNamed(name.clone()),
            CompositeKind::Union => TypeKind::UnionNamed(name.clone()),
        });
    };

    match &composite.kind {
        CompositeKind::Struct => {
            let mut fields = IndexMap::new();

            for member in members.iter() {
                match member {
                    MemberDeclaration::Member(member) => {
                        if !member.attributes.is_empty() {
                            todo!("attributes on members not supported yet");
                        }

                        for member_declarator in member.member_declarators.iter() {
                            match member_declarator {
                                MemberDeclarator::Declarator(declarator) => {
                                    let (name, ast_type, is_typedef) = get_name_and_type(
                                        typedefs,
                                        declarator,
                                        &DeclarationSpecifiers::from(&member.specifier_qualifiers),
                                        false,
                                    )?;

                                    fields.insert(
                                        name.clone(),
                                        Field {
                                            ast_type,
                                            privacy: Privacy::Public,
                                        },
                                    );
                                }
                                MemberDeclarator::BitField(_, _) => {
                                    todo!("bitfield members not supported yet")
                                }
                            }
                        }
                    }
                    MemberDeclaration::StaticAssert(_) => {
                        todo!("static assert as member in struct")
                    }
                }
            }

            let anonymous_struct = AnonymousStruct {
                fields,
                packed: false,
            };

            Ok(TypeKind::AnonymousStruct(anonymous_struct))
        }
        CompositeKind::Union => {
            todo!("union composites")
        }
    }
}

pub fn get_name_and_type(
    typedefs: &HashMap<String, CTypedef>,
    declarator: &Declarator,
    declaration_specifiers: &DeclarationSpecifiers,
    for_parameter: bool,
) -> Result<(String, Type, bool), ParseError> {
    let (name, decorators) = get_name_and_decorators(typedefs, declarator)?;
    let type_base = get_type_base(typedefs, declaration_specifiers, declarator.source)?;

    let mut ast_type = type_base.ast_type;

    for decorator in decorators.iter() {
        match decorator {
            Decorator::Pointer(pointer) => {
                ast_type = decorate_pointer(ast_type, pointer, decorator.source())?;
            }
            Decorator::Array(array) => {
                ast_type = decorate_array(ast_type, array, for_parameter, decorator.source())?;
            }
            Decorator::Function(function) => {
                ast_type = decorate_function(ast_type, function, decorator.source())?;
            }
        }
    }

    Ok((name, ast_type, type_base.is_typedef))
}

fn decorate_pointer(ast_type: Type, pointer: &Pointer, source: Source) -> Result<Type, ParseError> {
    if !pointer.type_qualifiers.is_empty() {
        eprintln!("warning: ignoring pointer type qualifiers");
    }

    Ok(Type::new(TypeKind::Pointer(Box::new(ast_type)), source))
}

fn decorate_array(
    ast_type: Type,
    array: &ArrayQualifier,
    for_parameter: bool,
    source: Source,
) -> Result<Type, ParseError> {
    if !array.type_qualifiers.is_empty() {
        todo!("array type qualifiers not supported yet");
    }

    if array.is_static {
        todo!("array static");
    }

    if array.is_param_vla {
        todo!("array get_name_and_type VLA");
    }

    if for_parameter {
        todo!("array get_name_and_type for parameter");
    } else {
        if let Some(count) = &array.expression {
            Ok(Type::new(
                TypeKind::FixedArray(Box::new(FixedArray {
                    ast_type,
                    count: translate_expr(count)?,
                })),
                source,
            ))
        } else {
            todo!("array get_name_and_type array non-parameter vla?");
        }
    }
}

fn decorate_function(
    ast_type: Type,
    function: &FunctionQualifier,
    source: Source,
) -> Result<Type, ParseError> {
    Ok(TypeKind::FunctionPointer(FunctionPointer {
        parameters: function.parameters.clone(),
        return_type: Box::new(ast_type),
        is_cstyle_variadic: function.is_cstyle_variadic,
    })
    .at(source))
}

fn get_name_and_decorators(
    typedefs: &HashMap<String, CTypedef>,
    declarator: &Declarator,
) -> Result<(String, Decorators), ParseError> {
    match &declarator.kind {
        DeclaratorKind::Named(name) => Ok((name.to_string(), Decorators::default())),
        DeclaratorKind::Pointer(inner, pointer) => {
            let (name, mut decorators) = get_name_and_decorators(typedefs, inner)?;
            decorators.then_pointer(pointer.clone());
            Ok((name, decorators))
        }
        DeclaratorKind::Function(inner, parameter_type_list) => {
            let (name, mut decorators) = get_name_and_decorators(typedefs, inner)?;
            let mut parameters =
                Vec::with_capacity(parameter_type_list.parameter_declarations.len());

            for parameter in parameter_type_list.parameter_declarations.iter() {
                let (parameter_name, parameter_type) = match &parameter.core {
                    ParameterDeclarationCore::Declarator(declarator) => {
                        let (parameter_name, ast_type, _) = get_name_and_type(
                            typedefs,
                            declarator,
                            &parameter.declaration_specifiers,
                            true,
                        )?;
                        (parameter_name, ast_type)
                    }
                    ParameterDeclarationCore::AbstractDeclarator(_) => todo!(),
                    ParameterDeclarationCore::Nothing => todo!(),
                };

                parameters.push(Parameter {
                    name: parameter_name,
                    ast_type: parameter_type,
                });
            }

            decorators.then_function(FunctionQualifier {
                parameters,
                source: declarator.source,
                is_cstyle_variadic: parameter_type_list.is_variadic,
            });

            Ok((name, decorators))
        }
        DeclaratorKind::Array(inner, array_qualifier) => {
            let (name, mut decorators) = get_name_and_decorators(typedefs, inner)?;
            decorators.then_array(array_qualifier.clone());
            Ok((name, decorators))
        }
    }
}
