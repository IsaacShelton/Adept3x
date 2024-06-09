#![allow(unreachable_code)]

mod error;
mod expr;
mod input;
mod speculate;
mod translation;

use self::speculate::speculate;
use self::{error::ParseErrorKind, expr::Expr, translation::declare_named};
use super::token::Integer;
use super::{
    punctuator::Punctuator,
    token::{CToken, CTokenKind},
};
use crate::ast::{Parameter, Type, TypeKind};
use crate::{
    ast::{Ast, FileIdentifier, Source},
    c::parser::translation::declare_function,
    source_file_cache::{SourceFileCache, SourceFileCacheKey},
};
use derive_more::IsVariant;
use itertools::Itertools;
use std::collections::HashMap;

pub use self::{error::ParseError, input::Input};

pub struct Parser<'a> {
    input: Input<'a>,
    typedefs: HashMap<String, CTypedef>,
    enum_constants: HashMap<String, Integer>,
}

#[derive(Clone, Debug)]
pub struct CTypedef {
    ast_type: Type,
}

#[derive(Clone, Debug)]
pub struct TypedefName {
    name: String,
    source: Source,
}

#[derive(Clone, Debug)]
pub struct ParameterTypeList {
    pub parameter_declarations: Vec<ParameterDeclaration>,
    pub is_variadic: bool,
}

#[derive(Clone, Debug, IsVariant)]
pub enum Abstraction {
    Normal,
    Abstract,
}

#[derive(Clone, Debug)]
pub struct Declarator {
    pub kind: DeclaratorKind,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum DeclaratorKind {
    Named(String),
    Pointer(Box<Declarator>, Pointer),
    Function(Box<Declarator>, ParameterTypeList),
    Array(Box<Declarator>, ArrayQualifier),
}

impl DeclaratorKind {
    pub fn at(self, source: Source) -> Declarator {
        Declarator { kind: self, source }
    }
}

#[derive(Clone, Debug)]
pub struct AbstractDeclarator {
    pub kind: AbstractDeclaratorKind,
    pub source: Source,
}

#[derive(Clone, Debug, IsVariant)]
pub enum AbstractDeclaratorKind {
    Nothing,
    Pointer(Box<AbstractDeclarator>, Pointer),
    Function(Box<AbstractDeclarator>, ParameterTypeList),
    Array(Box<AbstractDeclarator>, ArrayQualifier),
}

impl AbstractDeclaratorKind {
    pub fn at(self, source: Source) -> AbstractDeclarator {
        AbstractDeclarator { kind: self, source }
    }
}

#[derive(Clone, Debug, IsVariant)]
pub enum ParameterDeclarationCore {
    Declarator(Declarator),
    AbstractDeclarator(AbstractDeclarator),
    Nothing,
}

#[derive(Clone, Debug)]
pub struct ParameterDeclaration {
    pub attributes: Vec<()>,
    pub declaration_specifiers: DeclarationSpecifiers,
    pub core: ParameterDeclarationCore,
    pub source: Source,
}

#[derive(Clone, Debug, Default)]
pub struct Decorators {
    decorators: Vec<Decorator>,
}

#[derive(Clone, Debug)]
pub enum Decorator {
    Pointer(Pointer),
    Array(ArrayQualifier),
    Function(FunctionQualifier),
}

impl Decorator {
    pub fn source(&self) -> Source {
        match self {
            Decorator::Pointer(pointer) => pointer.source,
            Decorator::Array(array) => array.source,
            Decorator::Function(function) => function.source,
        }
    }
}

impl Decorators {
    pub fn then_pointer(&mut self, pointer: Pointer) {
        self.decorators.push(Decorator::Pointer(pointer))
    }

    pub fn then_array(&mut self, array: ArrayQualifier) {
        self.decorators.push(Decorator::Array(array))
    }

    pub fn then_function(&mut self, function: FunctionQualifier) {
        self.decorators.push(Decorator::Function(function));
    }

    pub fn iter(&self) -> impl Iterator<Item = &Decorator> {
        self.decorators.iter()
    }
}

#[derive(Clone, Debug)]
pub struct ArrayQualifier {
    expression: Option<Expr>,
    type_qualifiers: Vec<TypeQualifier>,
    is_static: bool,
    is_param_vla: bool,
    source: Source,
}

#[derive(Clone, Debug)]
pub struct FunctionQualifier {
    pub parameters: Vec<Parameter>,
    pub is_cstyle_variadic: bool,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct Pointer {
    pub attributes: Vec<()>,
    pub type_qualifiers: Vec<TypeQualifier>,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct InitDeclarator {
    pub declarator: Declarator,
    pub initializer: Option<()>,
}

#[derive(Clone, Debug)]
pub struct DeclarationSpecifier {
    kind: DeclarationSpecifierKind,
    source: Source,
}

impl From<TypeSpecifierQualifier> for DeclarationSpecifier {
    fn from(tsq: TypeSpecifierQualifier) -> Self {
        let source = tsq.source();

        DeclarationSpecifier {
            kind: DeclarationSpecifierKind::TypeSpecifierQualifier(tsq),
            source,
        }
    }
}

#[derive(Clone, Debug)]
pub enum DeclarationSpecifierKind {
    Auto,
    Constexpr,
    Extern,
    Register,
    Static,
    ThreadLocal,
    Typedef,
    Inline,
    Noreturn,
    TypeSpecifierQualifier(TypeSpecifierQualifier),
}

impl DeclarationSpecifierKind {
    pub fn at(self, source: Source) -> DeclarationSpecifier {
        DeclarationSpecifier { kind: self, source }
    }

    pub fn is_void(&self) -> bool {
        match self {
            Self::TypeSpecifierQualifier(tsq) => tsq.is_void(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub enum TypeSpecifierQualifier {
    TypeSpecifier(TypeSpecifier),
    TypeQualifier(TypeQualifier),
    AlignmentSpecifier(AlignmentSpecifier),
}

impl TypeSpecifierQualifier {
    pub fn source(&self) -> Source {
        match self {
            TypeSpecifierQualifier::TypeSpecifier(ts) => ts.source,
            TypeSpecifierQualifier::TypeQualifier(tq) => tq.source,
            TypeSpecifierQualifier::AlignmentSpecifier(al) => al.source,
        }
    }

    pub fn is_void(&self) -> bool {
        match self {
            TypeSpecifierQualifier::TypeSpecifier(ts) => ts.kind.is_void(),
            TypeSpecifierQualifier::TypeQualifier(_) => false,
            TypeSpecifierQualifier::AlignmentSpecifier(_) => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SpecifierQualifierList {
    pub attributes: Vec<()>,
    pub type_specifier_qualifiers: Vec<TypeSpecifierQualifier>,
}

#[derive(Clone, Debug)]
pub struct TypeSpecifier {
    kind: TypeSpecifierKind,
    source: Source,
}

#[derive(Clone, Debug, IsVariant)]
pub enum TypeSpecifierKind {
    Void,
    Bool,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
    Signed,
    Unsigned,
    Composite(Composite),
    Enumeration(Enumeration),
    TypedefName(TypedefName),
}

#[derive(Clone, Debug)]
pub struct TypeQualifier {
    pub kind: TypeQualifierKind,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum TypeQualifierKind {
    Const,
    Restrict,
    Volatile,
    Atomic,
}

#[derive(Clone, Debug)]
pub struct AlignmentSpecifier {
    pub kind: AlignmentSpecifierKind,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum AlignmentSpecifierKind {
    AlignAsType(()),
    AlisnAsConstExpr(()),
}

#[derive(Clone, Debug)]
pub struct DeclarationSpecifiers {
    pub specifiers: Vec<DeclarationSpecifier>,
    pub attributes: Vec<()>,
}

impl From<&SpecifierQualifierList> for DeclarationSpecifiers {
    fn from(value: &SpecifierQualifierList) -> Self {
        let specifiers = value
            .type_specifier_qualifiers
            .iter()
            .map(|tsq| DeclarationSpecifier::from(tsq.clone()))
            .collect_vec();

        Self {
            attributes: value.attributes.clone(),
            specifiers,
        }
    }
}

#[derive(Clone, Debug)]
pub enum MemberDeclaration {
    Member(Member),
    StaticAssert(StaticAssertDeclaration),
}

#[derive(Clone, Debug)]
pub struct StaticAssertDeclaration {
    pub condition: ConstantExpression,
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Member {
    pub attributes: Vec<()>,
    pub specifier_qualifiers: SpecifierQualifierList,
    pub member_declarators: Vec<MemberDeclarator>,
}

#[derive(Clone, Debug)]
pub enum MemberDeclarator {
    Declarator(Declarator),
    BitField(Option<Declarator>, ConstantExpression),
}

#[derive(Clone, Debug)]
pub enum ExternalDeclaration {
    Declaration(Declaration),
    FunctionDefinition(FunctionDefinition),
}

#[derive(Clone, Debug)]
pub enum Declaration {
    Common(CommonDeclaration),
    StaticAssert(StaticAssertDeclaration),
    Attribute(Vec<()>),
}

#[derive(Clone, Debug)]
pub struct CommonDeclaration {
    pub attribute_specifiers: Vec<()>,
    pub declaration_specifiers: DeclarationSpecifiers,
    pub init_declarator_list: Vec<InitDeclarator>,
}

#[derive(Clone, Debug)]
pub struct FunctionDefinition {}

#[derive(Clone, Debug)]
pub struct ConstantExpression {
    pub value: Expr,
}

#[derive(Clone, Debug)]
pub enum CompositeKind {
    Struct,
    Union,
}

#[derive(Clone, Debug)]
pub struct Composite {
    pub kind: CompositeKind,
    pub source: Source,
    pub name: Option<String>,
    pub attributes: Vec<()>,
    pub members: Option<Vec<MemberDeclaration>>,
}

#[derive(Clone, Debug)]
pub struct EnumTypeSpecifier {
    pub specifier_qualifier_list: SpecifierQualifierList,
}

#[derive(Clone, Debug)]
pub struct Enumerator {
    pub name: String,
    pub attributes: Vec<()>,
    pub value: Option<ConstantExpression>,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum Enumeration {
    Definition(EnumerationDefinition),
    Reference(EnumerationReference),
}

impl Enumeration {
    pub fn source(&self) -> Source {
        match self {
            Enumeration::Definition(definition) => definition.source,
            Enumeration::Reference(reference) => reference.source,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EnumerationDefinition {
    pub name: Option<String>,
    pub attributes: Vec<()>,
    pub enum_type_specifier: Option<EnumTypeSpecifier>,
    pub body: Vec<Enumerator>,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct EnumerationReference {
    pub name: String,
    pub enum_type_specifier: Option<EnumTypeSpecifier>,
    pub source: Source,
}

impl<'a> Parser<'a> {
    pub fn new(input: Input<'a>) -> Self {
        let mut typedefs = HashMap::default();

        eprintln!("warning: automatically inserting va_list definition");
        typedefs.insert(
            "va_list".into(),
            CTypedef {
                ast_type: Type::new(
                    TypeKind::Pointer(Box::new(Type::new(TypeKind::Void, Source::internal()))),
                    Source::internal(),
                ),
            },
        );

        Self {
            input,
            typedefs,
            enum_constants: HashMap::default(),
        }
    }

    pub fn parse(mut self) -> Result<Ast<'a>, ParseError> {
        // Get primary filename
        let filename = self.input.filename();

        // Create global ast
        let mut ast = Ast::new(filename.into(), self.input.source_file_cache());

        // Parse primary file
        self.parse_into(&mut ast, filename.into())?;

        // Return global ast
        Ok(ast)
    }

    pub fn parse_into(&mut self, ast: &mut Ast, filename: String) -> Result<(), ParseError> {
        // Create ast file
        let ast_file = ast.new_file(FileIdentifier::Local(filename));

        while !self.input.peek().is_end_of_file() {
            let external_declaration = self.parse_external_declaration()?;

            match external_declaration {
                ExternalDeclaration::Declaration(declaration) => match declaration {
                    Declaration::Common(declaration) => {
                        for init_declarator in declaration.init_declarator_list.iter() {
                            match &init_declarator.declarator.kind {
                                DeclaratorKind::Named(..)
                                | DeclaratorKind::Pointer(..)
                                | DeclaratorKind::Array(..) => declare_named(
                                    ast_file,
                                    &init_declarator.declarator,
                                    &declaration.attribute_specifiers[..],
                                    &declaration.declaration_specifiers,
                                    &mut self.typedefs,
                                )?,
                                DeclaratorKind::Function(declarator, parameter_type_list) => {
                                    declare_function(
                                        &mut self.typedefs,
                                        ast_file,
                                        &declaration.attribute_specifiers[..],
                                        &declaration.declaration_specifiers,
                                        declarator,
                                        parameter_type_list,
                                    )?;
                                }
                            }
                        }
                    }
                    Declaration::StaticAssert(_) => todo!(),
                    Declaration::Attribute(_) => todo!(),
                },
                ExternalDeclaration::FunctionDefinition(_) => todo!(),
            }
        }

        Ok(())
    }

    fn parse_external_declaration(&mut self) -> Result<ExternalDeclaration, ParseError> {
        if let Ok(_function_definition) = speculate!(self.input, self.parse_function_definition()) {
            return Ok(todo!());
        }

        return speculate!(self.input, self.parse_declaration());
    }

    fn parse_function_definition(&mut self) -> Result<(), ParseError> {
        self.parse_attribute_specifier_sequence()?;
        self.parse_declaration_specifiers()?;
        self.parse_declarator()?;
        self.parse_function_body()?;
        Ok(())
    }

    fn parse_attribute_specifier_sequence(&mut self) -> Result<Vec<()>, ParseError> {
        while self.eat_sequence(&[
            CTokenKind::Punctuator(Punctuator::OpenBracket),
            CTokenKind::Punctuator(Punctuator::OpenBracket),
        ]) {
            // TODO: Parse attribute
            unimplemented!("parsing c attributes");
        }

        Ok(vec![])
    }

    fn parse_declaration_specifiers(&mut self) -> Result<DeclarationSpecifiers, ParseError> {
        let mut specifiers = vec![];

        loop {
            match speculate!(self.input, self.parse_declaration_specifier()) {
                Ok(specifier) => specifiers.push(specifier),
                _ => break,
            }
        }

        let attributes = self.parse_attribute_specifier_sequence()?;
        Ok(DeclarationSpecifiers {
            specifiers,
            attributes,
        })
    }

    fn parse_declaration_specifier(&mut self) -> Result<DeclarationSpecifier, ParseError> {
        let CToken { kind, source } = self.input.peek();
        let source = *source;

        let result = match kind {
            CTokenKind::AutoKeyword => DeclarationSpecifierKind::Auto,
            CTokenKind::ConstexprKeyword => DeclarationSpecifierKind::Constexpr,
            CTokenKind::ExternKeyword => DeclarationSpecifierKind::Extern,
            CTokenKind::RegisterKeyword => DeclarationSpecifierKind::Register,
            CTokenKind::StaticKeyword => DeclarationSpecifierKind::Static,
            CTokenKind::ThreadLocalKeyword => DeclarationSpecifierKind::ThreadLocal,
            CTokenKind::TypedefKeyword => DeclarationSpecifierKind::Typedef,
            CTokenKind::InlineKeyword => DeclarationSpecifierKind::Inline,
            CTokenKind::NoreturnKeyword => DeclarationSpecifierKind::Noreturn,
            _ => {
                return Ok(DeclarationSpecifierKind::TypeSpecifierQualifier(
                    self.parse_type_specifier_qualifier()?,
                )
                .at(source));
            }
        };

        self.input.advance();
        Ok(result.at(source))
    }

    fn parse_type_specifier_qualifier(&mut self) -> Result<TypeSpecifierQualifier, ParseError> {
        if let Ok(type_specifier) = speculate!(self.input, self.parse_type_specifier()) {
            return Ok(TypeSpecifierQualifier::TypeSpecifier(type_specifier));
        }

        if let Ok(type_qualifier) = speculate!(self.input, self.parse_type_qualifier()) {
            return Ok(TypeSpecifierQualifier::TypeQualifier(type_qualifier));
        }

        if let Ok(alignment_specifier) = speculate!(self.input, self.parse_alignment_specifier()) {
            return Ok(TypeSpecifierQualifier::AlignmentSpecifier(
                alignment_specifier,
            ));
        }

        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse type specifier qualifier"),
            self.input.peek().source,
        ))
    }

    fn parse_type_specifier(&mut self) -> Result<TypeSpecifier, ParseError> {
        if let Some(type_specifier_kind) = match self.input.peek().kind {
            CTokenKind::Decimal32Keyword => unimplemented!("_Decimal32"),
            CTokenKind::Decimal64Keyword => unimplemented!("_Decimal64"),
            CTokenKind::Decimal128Keyword => unimplemented!("_Decimal128"),
            CTokenKind::ComplexKeyword => unimplemented!("_Complex"),
            CTokenKind::BitIntKeyword => unimplemented!("_BitInt"),
            CTokenKind::BoolKeyword => Some(TypeSpecifierKind::Bool),
            CTokenKind::VoidKeyword => Some(TypeSpecifierKind::Void),
            CTokenKind::CharKeyword => Some(TypeSpecifierKind::Char),
            CTokenKind::ShortKeyword => Some(TypeSpecifierKind::Short),
            CTokenKind::IntKeyword => Some(TypeSpecifierKind::Int),
            CTokenKind::LongKeyword => Some(TypeSpecifierKind::Long),
            CTokenKind::FloatKeyword => Some(TypeSpecifierKind::Float),
            CTokenKind::DoubleKeyword => Some(TypeSpecifierKind::Double),
            CTokenKind::SignedKeyword => Some(TypeSpecifierKind::Signed),
            CTokenKind::UnsignedKeyword => Some(TypeSpecifierKind::Unsigned),
            _ => None,
        } {
            let source = self.input.advance().source;

            return Ok(TypeSpecifier {
                kind: type_specifier_kind,
                source,
            });
        }

        let source = self.input.peek().source;

        if let Ok(..) = speculate!(self.input, self.parse_atomic_type_specifier()) {
            return Ok(todo!());
        }

        if let Ok(composite) = speculate!(self.input, self.parse_struct_or_union_specifier()) {
            return Ok(TypeSpecifier {
                kind: TypeSpecifierKind::Composite(composite),
                source,
            });
        }

        if let Ok(enumeration) = speculate!(self.input, self.parse_enum_specifier()) {
            return Ok(TypeSpecifier {
                kind: TypeSpecifierKind::Enumeration(enumeration),
                source,
            });
        }

        if let Ok(typedef_name) = speculate!(self.input, self.parse_typedef_name()) {
            return Ok(TypeSpecifier {
                kind: TypeSpecifierKind::TypedefName(typedef_name),
                source,
            });
        }

        if let Ok(..) = speculate!(self.input, self.parse_typeof_specifier()) {
            return Ok(todo!());
        }

        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse type specifier"),
            self.input.peek().source,
        ))
    }

    fn parse_type_qualifier(&mut self) -> Result<TypeQualifier, ParseError> {
        let kind = match self.input.peek().kind {
            CTokenKind::ConstKeyword => TypeQualifierKind::Const,
            CTokenKind::RestrictKeyword => TypeQualifierKind::Restrict,
            CTokenKind::VolatileKeyword => TypeQualifierKind::Volatile,
            CTokenKind::AtomicKeyword => TypeQualifierKind::Atomic,
            _ => {
                return Err(ParseError::new(
                    ParseErrorKind::Misc("Failed to parse type qualifier"),
                    self.input.peek().source,
                ))
            }
        };

        let source = self.input.advance().source;
        Ok(TypeQualifier { kind, source })
    }

    fn parse_alignment_specifier(&mut self) -> Result<AlignmentSpecifier, ParseError> {
        if !self.eat(CTokenKind::AlignasKeyword) {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected 'alignas' keyword to begin alignment specifier"),
                self.input.peek().source,
            ));
        }

        if !self.eat_open_paren() {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected '(' after 'alignas' keyword"),
                self.input.peek().source,
            ));
        }

        todo!()
    }

    fn parse_declarator(&mut self) -> Result<Declarator, ParseError> {
        let pointers = self.parse_pointers()?;
        let declarator = self.parse_direct_declarator(pointers)?;
        Ok(declarator)
    }

    fn parse_abstract_declarator(&mut self) -> Result<AbstractDeclarator, ParseError> {
        let pointers = self.parse_pointers()?;
        let declarator = self.parse_direct_abstract_declarator_or_nothing(pointers)?;

        if declarator.kind.is_nothing() {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected abstract declarator"),
                self.input.peek().source,
            ));
        }

        Ok(declarator)
    }

    fn parse_pointers(&mut self) -> Result<Vec<Pointer>, ParseError> {
        let mut pointers = vec![];

        while let Some(source) = self.eat_punctuator_source(Punctuator::Multiply) {
            let attributes = self.parse_attribute_specifier_sequence()?;

            let type_qualifiers = self.parse_type_qualifier_list()?;
            pointers.push(Pointer {
                attributes,
                type_qualifiers,
                source,
            });
        }

        Ok(pointers)
    }

    fn parse_type_qualifier_list(&mut self) -> Result<Vec<TypeQualifier>, ParseError> {
        let mut qualifiers = vec![];

        loop {
            match speculate!(self.input, self.parse_type_qualifier()) {
                Ok(qualifier) => qualifiers.push(qualifier),
                _ => break,
            }
        }

        Ok(qualifiers)
    }

    fn parse_direct_declarator(
        &mut self,
        mut pointers: Vec<Pointer>,
    ) -> Result<Declarator, ParseError> {
        let mut declarator = if let Some((name, source)) = self.eat_identifier_source() {
            let _attributes = self.parse_attribute_specifier_sequence()?;
            DeclaratorKind::Named(name).at(source)
        } else if self.eat_open_paren() {
            let declarator = self.parse_declarator()?;

            if !self.eat_punctuator(Punctuator::CloseParen) {
                return Err(ParseError::new(
                    ParseErrorKind::Misc("Expected ')' to close nested direct declarator"),
                    self.input.peek().source,
                ));
            }

            declarator
        } else {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected declarator"),
                self.input.peek().source,
            ));
        };

        for pointer in pointers.drain(..) {
            let source = pointer.source;
            declarator = DeclaratorKind::Pointer(Box::new(declarator), pointer).at(source);
        }

        loop {
            match &self.input.peek().kind {
                CTokenKind::Punctuator(Punctuator::OpenBracket) => {
                    declarator = self.parse_array_declarator(declarator)?;
                    let _attributes = self.parse_attribute_specifier_sequence()?;
                }
                CTokenKind::Punctuator(Punctuator::OpenParen { .. }) => {
                    declarator = self.parse_function_declarator(declarator)?;
                    let _attributes = self.parse_attribute_specifier_sequence()?;
                }
                _ => break,
            }
        }

        Ok(declarator)
    }

    fn parse_direct_abstract_declarator_or_nothing(
        &mut self,
        mut pointers: Vec<Pointer>,
    ) -> Result<AbstractDeclarator, ParseError> {
        let mut abstract_declarator = if self.eat_open_paren() {
            let declarator = self.parse_abstract_declarator()?;

            if !self.eat_punctuator(Punctuator::CloseParen) {
                return Err(ParseError::new(
                    ParseErrorKind::Misc("Expected ')' to close nested direct abstract declarator"),
                    self.input.peek().source,
                ));
            }

            declarator
        } else {
            AbstractDeclaratorKind::Nothing.at(self.input.peek().source)
        };

        for pointer in pointers.drain(..) {
            let source = pointer.source;
            abstract_declarator =
                AbstractDeclaratorKind::Pointer(Box::new(abstract_declarator), pointer).at(source);
        }

        loop {
            match &self.input.peek().kind {
                CTokenKind::Punctuator(Punctuator::OpenBracket) => {
                    abstract_declarator =
                        self.parse_abstract_array_declarator(abstract_declarator)?;
                    let _attributes = self.parse_attribute_specifier_sequence()?;
                }
                CTokenKind::Punctuator(Punctuator::OpenParen { .. }) => {
                    abstract_declarator =
                        self.parse_abstract_function_declarator(abstract_declarator)?;
                    let _attributes = self.parse_attribute_specifier_sequence()?;
                }
                _ => break,
            }
        }

        Ok(abstract_declarator)
    }

    fn parse_array_qualifier(
        &mut self,
        abstraction: Abstraction,
    ) -> Result<ArrayQualifier, ParseError> {
        let source = self.input.peek().source;

        if !self.eat_punctuator(Punctuator::OpenBracket) {
            return Err(ParseErrorKind::Misc("Expected '[' to begin array declarator").at(source));
        }

        let mut is_static = self.eat(CTokenKind::StaticKeyword);
        let type_qualifiers = self.parse_type_qualifier_list()?;

        if !is_static {
            is_static = self.eat(CTokenKind::StaticKeyword);
        }

        let expression = speculate!(self.input, self.parse_expr_singular()).ok();
        let is_param_vla = expression.is_none() && self.eat_punctuator(Punctuator::Multiply);

        if !self.eat_punctuator(Punctuator::CloseBracket) {
            return Err(ParseErrorKind::Misc("Expected ']' to close array declarator").at(source));
        }

        if is_param_vla && !type_qualifiers.is_empty() && abstraction.is_abstract() {
            return Err(ParseErrorKind::Misc(
                "Cannot specify type qualifiers for abstract array declarator",
            )
            .at(source));
        }

        Ok(ArrayQualifier {
            expression,
            type_qualifiers,
            is_static,
            is_param_vla,
            source,
        })
    }

    fn parse_array_declarator(&mut self, declarator: Declarator) -> Result<Declarator, ParseError> {
        let source = self.input.peek().source;
        let array_qualifier = self.parse_array_qualifier(Abstraction::Normal)?;
        Ok(DeclaratorKind::Array(Box::new(declarator), array_qualifier).at(source))
    }

    fn parse_abstract_array_declarator(
        &mut self,
        declarator: AbstractDeclarator,
    ) -> Result<AbstractDeclarator, ParseError> {
        let source = self.input.peek().source;
        let array_qualifier = self.parse_array_qualifier(Abstraction::Abstract)?;
        Ok(AbstractDeclaratorKind::Array(Box::new(declarator), array_qualifier).at(source))
    }

    fn parse_function_declarator(
        &mut self,
        declarator: Declarator,
    ) -> Result<Declarator, ParseError> {
        let source = self.input.peek().source;

        if !self.eat_open_paren() {
            return Err(
                ParseErrorKind::Misc("Expected '(' to begin function declarator").at(source),
            );
        }

        let parameter_type_list = self.parse_parameter_type_list()?;

        if !self.eat_punctuator(Punctuator::CloseParen) {
            return Err(
                ParseErrorKind::Misc("Expected ')' to close function declarator").at(source),
            );
        }

        Ok(DeclaratorKind::Function(Box::new(declarator), parameter_type_list).at(source))
    }

    fn parse_abstract_function_declarator(
        &mut self,
        declarator: AbstractDeclarator,
    ) -> Result<AbstractDeclarator, ParseError> {
        let source = self.input.peek().source;

        if !self.eat_open_paren() {
            return Err(
                ParseErrorKind::Misc("Expected '(' to begin abstract function declarator")
                    .at(source),
            );
        }

        let parameter_type_list = self.parse_parameter_type_list()?;

        if !self.eat_punctuator(Punctuator::CloseParen) {
            return Err(
                ParseErrorKind::Misc("Expected ')' to close abstract function declarator")
                    .at(source),
            );
        }

        Ok(AbstractDeclaratorKind::Function(Box::new(declarator), parameter_type_list).at(source))
    }

    fn parse_parameter_type_list(&mut self) -> Result<ParameterTypeList, ParseError> {
        let mut parameter_declarations = Vec::new();
        let mut is_variadic = false;

        loop {
            if let CTokenKind::Punctuator(Punctuator::Ellipses) = self.input.peek().kind {
                self.input.advance();
                is_variadic = true;
                break;
            }

            let declaration = self.parse_parameter_declaration()?;
            parameter_declarations.push(declaration);

            match self.input.peek().kind {
                CTokenKind::Punctuator(Punctuator::CloseParen) => break,
                CTokenKind::Punctuator(Punctuator::Comma) => {
                    let _ = self.input.advance();
                }
                _ => {
                    return Err(ParseError::new(
                        ParseErrorKind::Misc(
                            "Expected ',' after parameter declaration in parameter type list",
                        ),
                        self.input.peek().source,
                    ))
                }
            }
        }

        Ok(ParameterTypeList {
            parameter_declarations,
            is_variadic,
        })
    }

    fn parse_parameter_declaration(&mut self) -> Result<ParameterDeclaration, ParseError> {
        let source = self.input.peek().source;
        let attributes = self.parse_attribute_specifier_sequence()?;
        let declaration_specifiers = self.parse_declaration_specifiers()?;

        if let Ok(declarator) = speculate!(self.input, self.parse_declarator()) {
            return Ok(ParameterDeclaration {
                attributes,
                declaration_specifiers,
                core: ParameterDeclarationCore::Declarator(declarator),
                source,
            });
        }

        if let Ok(abstract_declarator) = speculate!(self.input, self.parse_abstract_declarator()) {
            return Ok(ParameterDeclaration {
                attributes,
                declaration_specifiers,
                core: ParameterDeclarationCore::AbstractDeclarator(abstract_declarator),
                source,
            });
        }

        Ok(ParameterDeclaration {
            attributes,
            declaration_specifiers,
            core: ParameterDeclarationCore::Nothing,
            source,
        })
    }

    fn parse_atomic_type_specifier(&mut self) -> Result<(), ParseError> {
        if self.input.peek_is(CTokenKind::AtomicKeyword) {
            todo!()
        }

        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse atomic specifier"),
            self.input.peek().source,
        ))
    }

    fn parse_struct_or_union_specifier(&mut self) -> Result<Composite, ParseError> {
        let kind = match self.input.peek().kind {
            CTokenKind::StructKeyword => CompositeKind::Struct,
            CTokenKind::UnionKeyword => CompositeKind::Union,
            _ => {
                return Err(
                    ParseErrorKind::Misc("Failed to parse struct or union specifier")
                        .at(self.input.peek().source),
                )
            }
        };

        let source = self.input.advance().source;
        let attributes = self.parse_attribute_specifier_sequence()?;

        let name = if self.input.peek().kind.is_identifier() {
            Some(self.input.advance().kind.clone().unwrap_identifier())
        } else {
            None
        };

        let members = speculate!(
            self.input,
            self.parse_member_declaration_list_including_braces()
        )
        .ok();

        if name.is_none() && members.is_none() {
            return Err(ParseErrorKind::ExpectedTypeNameOrMemberDeclarationList
                .at(self.input.peek().source));
        }

        Ok(Composite {
            kind,
            source,
            name,
            attributes,
            members,
        })
    }

    fn parse_member_declaration_list_including_braces(
        &mut self,
    ) -> Result<Vec<MemberDeclaration>, ParseError> {
        if !self.eat_punctuator(Punctuator::OpenCurly) {
            return Err(
                ParseErrorKind::Misc("Expected '{' to begin member declaration list")
                    .at(self.input.peek().source),
            );
        }

        let mut member_declarations = vec![];

        while !matches!(
            self.input.peek().kind,
            CTokenKind::EndOfFile | CTokenKind::Punctuator(Punctuator::CloseCurly)
        ) {
            member_declarations.push(self.parse_member_declaration()?);
        }

        if !self.eat_punctuator(Punctuator::CloseCurly) {
            return Err(
                ParseErrorKind::Misc("Expected '}' to close member declaration list")
                    .at(self.input.peek().source),
            );
        }

        Ok(member_declarations)
    }

    fn parse_member_declaration(&mut self) -> Result<MemberDeclaration, ParseError> {
        if self.input.peek().kind.is_static_assert_keyword() {
            return Ok(MemberDeclaration::StaticAssert(self.parse_static_assert()?));
        }

        let attributes = self.parse_attribute_specifier_sequence()?;
        let specifier_qualifiers = self.parse_specifier_qualifier_list()?;
        let member_declarations = self.parse_member_declarator_list()?;

        if !self.eat_punctuator(Punctuator::Semicolon) {
            return Err(ParseErrorKind::ExpectedSemicolon.at(self.input.peek().source));
        }

        Ok(MemberDeclaration::Member(Member {
            attributes,
            specifier_qualifiers,
            member_declarators: member_declarations,
        }))
    }

    fn parse_member_declarator_list(&mut self) -> Result<Vec<MemberDeclarator>, ParseError> {
        let mut member_declarators = vec![];

        loop {
            if member_declarators.len() != 0 {
                if !self.eat_punctuator(Punctuator::Comma) {
                    break;
                }
            }

            match speculate!(self.input, self.parse_member_declarator()) {
                Ok(member_declarator) => member_declarators.push(member_declarator),
                _ => break,
            }
        }

        Ok(member_declarators)
    }

    fn parse_member_declarator(&mut self) -> Result<MemberDeclarator, ParseError> {
        let declarator = if !self
            .input
            .peek_is(CTokenKind::Punctuator(Punctuator::Colon))
        {
            Some(self.parse_declarator()?)
        } else {
            None
        };

        let bits = if self.eat_punctuator(Punctuator::Colon) {
            Some(self.parse_constant_expression()?)
        } else {
            None
        };

        if let Some(bits) = bits {
            Ok(MemberDeclarator::BitField(declarator, bits))
        } else if let Some(declarator) = declarator {
            Ok(MemberDeclarator::Declarator(declarator))
        } else {
            Err(ParseErrorKind::ExpectedMemberDeclarator.at(self.input.peek().source))
        }
    }

    fn parse_constant_expression(&mut self) -> Result<ConstantExpression, ParseError> {
        let value = self.parse_expr_singular()?;

        eprintln!("warning: constant expressions are not validated to not contain '='");

        Ok(ConstantExpression { value })
    }

    fn parse_specifier_qualifier_list(&mut self) -> Result<SpecifierQualifierList, ParseError> {
        let mut type_specifier_qualifiers = vec![];

        loop {
            match speculate!(self.input, self.parse_type_specifier_qualifier()) {
                Ok(qualifier) => type_specifier_qualifiers.push(qualifier),
                _ => break,
            }
        }

        let attributes = self.parse_attribute_specifier_sequence()?;

        Ok(SpecifierQualifierList {
            attributes,
            type_specifier_qualifiers,
        })
    }

    fn parse_typedef_name(&mut self) -> Result<TypedefName, ParseError> {
        if let CTokenKind::Identifier(name) = &self.input.peek().kind {
            if self.typedefs.get(name).is_some() {
                let name = name.clone();
                let source = self.input.advance().source;
                return Ok(TypedefName { name, source });
            }
        }

        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse typedef name"),
            self.input.peek().source,
        ))
    }

    fn parse_enum_specifier(&mut self) -> Result<Enumeration, ParseError> {
        let source = self.input.peek().source;

        if !self.eat(CTokenKind::EnumKeyword) {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Failed to parse enum specifier"),
                source,
            ));
        }

        let enum_start_of_head = self.input.peek().source;
        let attributes = self.parse_attribute_specifier_sequence()?;
        let name = self.eat_identifier();
        let enum_type_specifier = self.parse_enum_type_specifier()?;
        let body = self.parse_enum_body()?;

        if let Some(body) = body {
            // Enum definition
            Ok(Enumeration::Definition(EnumerationDefinition {
                name,
                attributes,
                enum_type_specifier,
                body,
                source,
            }))
        } else if let Some(name) = name {
            // Enum reference
            if !attributes.is_empty() {
                return Err(ParseError::new(
                    ParseErrorKind::Misc("Cannot specify attributes on enum reference"),
                    enum_start_of_head,
                ));
            }

            Ok(Enumeration::Reference(EnumerationReference {
                name,
                enum_type_specifier,
                source,
            }))
        } else {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected name of enum"),
                enum_start_of_head,
            ));
        }
    }

    fn parse_enum_type_specifier(&mut self) -> Result<Option<EnumTypeSpecifier>, ParseError> {
        if !self.eat_punctuator(Punctuator::Colon) {
            return Ok(None);
        }

        Ok(Some(EnumTypeSpecifier {
            specifier_qualifier_list: self.parse_specifier_qualifier_list()?,
        }))
    }

    fn parse_enum_body(&mut self) -> Result<Option<Vec<Enumerator>>, ParseError> {
        if !self.eat_punctuator(Punctuator::OpenCurly) {
            return Ok(None);
        }

        let mut enumerators = vec![];

        while !self
            .input
            .peek_is_or_eof(CTokenKind::Punctuator(Punctuator::CloseCurly))
        {
            enumerators.push(self.parse_enumerator()?);

            if !self.eat_punctuator(Punctuator::Comma)
                && !self
                    .input
                    .peek_is(CTokenKind::Punctuator(Punctuator::CloseCurly))
            {
                return Err(ParseErrorKind::Misc("Expected ',' or '}' after enumerator")
                    .at(self.input.peek().source));
            }
        }

        if !self.eat_punctuator(Punctuator::CloseCurly) {
            return Err(ParseErrorKind::Misc("Expected '}' after enumeration body")
                .at(self.input.peek().source));
        }

        Ok(Some(enumerators))
    }

    fn parse_enumerator(&mut self) -> Result<Enumerator, ParseError> {
        let (name, source) = self.eat_identifier_source().ok_or_else(|| {
            ParseErrorKind::Misc("Expected name of enumerator inside enumeration body")
                .at(self.input.peek().source)
        })?;

        let attributes = self.parse_attribute_specifier_sequence()?;

        let value = if self.eat_punctuator(Punctuator::Assign) {
            Some(self.parse_constant_expression()?)
        } else {
            None
        };

        Ok(Enumerator {
            name,
            attributes,
            value,
            source,
        })
    }

    fn parse_typeof_specifier(&mut self) -> Result<(), ParseError> {
        match self.input.advance().kind {
            CTokenKind::TypeofKeyword => todo!(),
            CTokenKind::TypeofUnqualKeyword => todo!(),
            _ => Err(ParseError::new(
                ParseErrorKind::Misc("Failed to parse typeof specifier"),
                self.input.peek().source,
            )),
        }
    }

    fn parse_static_assert(&mut self) -> Result<StaticAssertDeclaration, ParseError> {
        if !self.eat(CTokenKind::StaticAssertKeyword) {
            return Err(ParseErrorKind::Misc(
                "Expected 'static_assert' keyword to begin static assert",
            )
            .at(self.input.peek().source));
        }

        todo!()
    }

    fn parse_declaration(&mut self) -> Result<ExternalDeclaration, ParseError> {
        if self.input.peek_is(CTokenKind::StaticAssertKeyword) {
            return Ok(ExternalDeclaration::Declaration(Declaration::StaticAssert(
                self.parse_static_assert()?,
            )));
        }

        let attribute_specifiers = self.parse_attribute_specifier_sequence()?;

        if self.eat_punctuator(Punctuator::Semicolon) {
            // attribute-declaration
            todo!();
            return Ok(todo!());
        }

        let declaration_specifiers = self.parse_declaration_specifiers()?;
        let init_declarator_list = self.parse_init_declarator_list()?;

        if !attribute_specifiers.is_empty() && init_declarator_list.is_empty() {
            return Err(ParseError::new(
                ParseErrorKind::Misc(
                    "Expected at least one init declarator when attribute specifiers present",
                ),
                self.input.peek().source,
            ));
        }

        if !self.eat_punctuator(Punctuator::Semicolon) {
            // TODO: Improve error message
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected ';' after declaration"),
                self.input.peek().source,
            ));
        }

        return Ok(ExternalDeclaration::Declaration(Declaration::Common(
            CommonDeclaration {
                attribute_specifiers,
                declaration_specifiers,
                init_declarator_list,
            },
        )));
    }

    fn parse_init_declarator_list(&mut self) -> Result<Vec<InitDeclarator>, ParseError> {
        let mut list = vec![];

        loop {
            match speculate!(self.input, self.parse_init_declarator()) {
                Ok(init_declarator) => list.push(init_declarator),
                Err(_) => return Ok(list),
            }

            if !self.eat_punctuator(Punctuator::Comma) {
                break;
            }
        }

        Ok(list)
    }

    fn parse_init_declarator(&mut self) -> Result<InitDeclarator, ParseError> {
        let declarator = self.parse_declarator()?;

        let initializer = if self.eat_punctuator(Punctuator::Assign) {
            Some(self.parse_initializer()?)
        } else {
            None
        };

        Ok(InitDeclarator {
            declarator,
            initializer,
        })
    }

    fn parse_initializer(&mut self) -> Result<(), ParseError> {
        if let Ok(..) = speculate!(self.input, self.parse_braced_initializer()) {
            todo!();
            return Ok(());
        }

        if let Ok(..) = speculate!(self.input, self.parse_expr_singular()) {
            todo!();
            return Ok(());
        }

        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse initializer"),
            self.input.peek().source,
        ))
    }

    fn parse_braced_initializer(&mut self) -> Result<(), ParseError> {
        if !self.eat_punctuator(Punctuator::OpenCurly) {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected '{' to begin braced initializer"),
                self.input.peek().source,
            ));
        }

        todo!();

        if !self.eat_punctuator(Punctuator::CloseCurly) {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected '}' to close braced initializer"),
                self.input.peek().source,
            ));
        }

        todo!();
        Ok(())
    }

    fn parse_function_body(&mut self) -> Result<(), ParseError> {
        self.parse_compound_statement()
    }

    fn parse_compound_statement(&mut self) -> Result<(), ParseError> {
        if !self.eat_punctuator(Punctuator::OpenCurly) {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected '{' to begin compound statement"),
                self.input.peek().source,
            ));
        }

        todo!();

        if !self.eat_punctuator(Punctuator::CloseCurly) {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected '}' to close compound statement"),
                self.input.peek().source,
            ));
        }

        Ok(())
    }

    fn eat(&mut self, expected: CTokenKind) -> bool {
        if self.input.peek().kind == expected {
            self.input.advance();
            true
        } else {
            false
        }
    }

    fn eat_identifier(&mut self) -> Option<String> {
        match &self.input.peek().kind {
            CTokenKind::Identifier(name) => {
                let name = name.clone();
                self.input.advance();
                Some(name)
            }
            _ => None,
        }
    }

    fn eat_identifier_source(&mut self) -> Option<(String, Source)> {
        match &self.input.peek().kind {
            CTokenKind::Identifier(name) => {
                let name = name.clone();
                let source = self.input.advance().source;
                Some((name, source))
            }
            _ => None,
        }
    }

    fn eat_punctuator(&mut self, expected: Punctuator) -> bool {
        if self.input.peek().kind == CTokenKind::Punctuator(expected) {
            self.input.advance();
            true
        } else {
            false
        }
    }

    fn eat_punctuator_source(&mut self, expected: Punctuator) -> Option<Source> {
        if self.input.peek().kind == CTokenKind::Punctuator(expected) {
            Some(self.input.advance().source)
        } else {
            None
        }
    }

    fn eat_open_paren(&mut self) -> bool {
        if let CTokenKind::Punctuator(Punctuator::OpenParen { .. }) = self.input.peek().kind {
            self.input.advance();
            true
        } else {
            false
        }
    }

    fn eat_sequence(&mut self, expected: &[CTokenKind]) -> bool {
        self.eat_sequence_source(expected).is_some()
    }

    fn eat_sequence_source(&mut self, expected: &[CTokenKind]) -> Option<Source> {
        for (i, expected_kind) in expected.iter().enumerate() {
            if self.input.peek_nth(i).kind != *expected_kind {
                return None;
            }
        }

        let source = if expected.len() > 0 {
            Some(self.input.peek().source)
        } else {
            None
        };

        for _ in 0..expected.len() {
            self.input.advance();
        }

        source
    }
}

pub fn parse(
    tokens: Vec<CToken>,
    source_file_cache: &SourceFileCache,
    key: SourceFileCacheKey,
) -> Result<Ast, ParseError> {
    Parser::new(Input::new(tokens, source_file_cache, key)).parse()
}

pub fn parse_into(
    tokens: Vec<CToken>,
    source_file_cache: &SourceFileCache,
    key: SourceFileCacheKey,
    ast: &mut Ast,
    filename: String,
) -> Result<(), ParseError> {
    Parser::new(Input::new(tokens, source_file_cache, key)).parse_into(ast, filename)
}
