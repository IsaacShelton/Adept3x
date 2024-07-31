mod composite;
mod decorate;
mod enumeration;
mod get_type_base;

use self::composite::make_composite;
use self::decorate::{decorate_array, decorate_function, decorate_pointer};
use self::enumeration::make_anonymous_enum;
use self::get_type_base::get_type_base;
use super::parameters::has_parameters;
use crate::{
    ast::{AstFile, FloatSize, IntegerBits, IntegerSign, Parameter, Source, Type, TypeKind},
    c::parser::{
        error::ParseErrorKind, AlignmentSpecifierKind, CTypedef, DeclarationSpecifiers, Declarator,
        DeclaratorKind, Decorator, Decorators, FunctionQualifier, ParameterDeclarationCore,
        ParseError, TypeQualifierKind, TypeSpecifierKind, TypeSpecifierQualifier,
    },
    diagnostics::Diagnostics,
};
use std::collections::HashMap;

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

pub fn build_type_specifier_qualifier(
    ast_file: &mut AstFile,
    builder: &mut TypeBaseBuilder,
    typedefs: &HashMap<String, CTypedef>,
    tsq: &TypeSpecifierQualifier,
    diagnostics: &Diagnostics,
) -> Result<(), ParseError> {
    match tsq {
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
            TypeSpecifierKind::Unsigned => builder.sign(IntegerSign::Unsigned, ts.source)?,
            TypeSpecifierKind::Composite(composite) => builder.concrete(
                make_composite(ast_file, typedefs, composite, diagnostics)?,
                ts.source,
            )?,
            TypeSpecifierKind::Enumeration(enumeration) => {
                builder.concrete(make_anonymous_enum(ast_file, enumeration)?, ts.source)?
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
    }

    Ok(())
}

pub fn get_name_and_type(
    ast_file: &mut AstFile,
    typedefs: &HashMap<String, CTypedef>,
    declarator: &Declarator,
    declaration_specifiers: &DeclarationSpecifiers,
    for_parameter: bool,
    diagnostics: &Diagnostics,
) -> Result<(String, Type, bool), ParseError> {
    let (name, decorators) = get_name_and_decorators(ast_file, typedefs, declarator, diagnostics)?;
    let type_base = get_type_base(
        ast_file,
        typedefs,
        declaration_specifiers,
        declarator.source,
        diagnostics,
    )?;

    let mut ast_type = type_base.ast_type;

    for decorator in decorators.iter() {
        match decorator {
            Decorator::Pointer(pointer) => {
                ast_type = decorate_pointer(ast_type, pointer, decorator.source(), diagnostics)?;
            }
            Decorator::Array(array) => {
                ast_type = decorate_array(
                    ast_file,
                    typedefs,
                    ast_type,
                    array,
                    for_parameter,
                    decorator.source(),
                    diagnostics,
                )?;
            }
            Decorator::Function(function) => {
                ast_type = decorate_function(ast_type, function, decorator.source())?;
            }
        }
    }

    Ok((name, ast_type, type_base.is_typedef))
}

fn get_name_and_decorators(
    ast_file: &mut AstFile,
    typedefs: &HashMap<String, CTypedef>,
    declarator: &Declarator,
    diagnostics: &Diagnostics,
) -> Result<(String, Decorators), ParseError> {
    match &declarator.kind {
        DeclaratorKind::Named(name) => Ok((name.to_string(), Decorators::default())),
        DeclaratorKind::Pointer(inner, pointer) => {
            let (name, mut decorators) =
                get_name_and_decorators(ast_file, typedefs, inner, diagnostics)?;
            decorators.then_pointer(pointer.clone());
            Ok((name, decorators))
        }
        DeclaratorKind::Function(inner, parameter_type_list) => {
            let (name, mut decorators) =
                get_name_and_decorators(ast_file, typedefs, inner, diagnostics)?;
            let mut parameters =
                Vec::with_capacity(parameter_type_list.parameter_declarations.len());

            if has_parameters(parameter_type_list) {
                for parameter in parameter_type_list.parameter_declarations.iter() {
                    let (parameter_name, parameter_type) = match &parameter.core {
                        ParameterDeclarationCore::Declarator(declarator) => {
                            let (parameter_name, ast_type, _) = get_name_and_type(
                                ast_file,
                                typedefs,
                                declarator,
                                &parameter.declaration_specifiers,
                                true,
                                diagnostics,
                            )?;
                            (parameter_name, ast_type)
                        }
                        ParameterDeclarationCore::AbstractDeclarator(_) => todo!(),
                        ParameterDeclarationCore::Nothing => {
                            todo!()
                        }
                    };

                    parameters.push(Parameter {
                        name: parameter_name,
                        ast_type: parameter_type,
                    });
                }
            }

            decorators.then_function(FunctionQualifier {
                parameters,
                source: declarator.source,
                is_cstyle_variadic: parameter_type_list.is_variadic,
            });

            Ok((name, decorators))
        }
        DeclaratorKind::Array(inner, array_qualifier) => {
            let (name, mut decorators) =
                get_name_and_decorators(ast_file, typedefs, inner, diagnostics)?;
            decorators.then_array(array_qualifier.clone());
            Ok((name, decorators))
        }
    }
}
