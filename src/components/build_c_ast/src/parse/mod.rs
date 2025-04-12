#![allow(unreachable_code)]

pub mod error;
pub mod expr;
mod input;
mod speculate;

pub use self::{error::ParseError, input::Input};
use self::{error::ParseErrorKind, speculate::speculate};
use crate::{
    CFileType,
    translate::{TranslateCtx, declare_function, declare_named_declaration},
};
use c_ast::*;
use c_token::{CToken, CTokenKind, Integer, Punctuator};
use diagnostics::{Diagnostics, WarningDiagnostic};
use source_files::Source;
use std::collections::HashMap;

pub struct Parser<'input, 'diagnostics> {
    input: Input<'input>,
    pub ast_file: ast::RawAstFile,
    pub typedefs: HashMap<String, CTypedef>,
    pub diagnostics: &'diagnostics Diagnostics<'diagnostics>,
    enum_constants: HashMap<String, Integer>,
    c_file_type: CFileType,
}

impl<'input, 'diagnostics> Parser<'input, 'diagnostics> {
    pub fn new(
        input: Input<'input>,
        diagnostics: &'diagnostics Diagnostics<'diagnostics>,
        c_file_type: CFileType,
    ) -> Self {
        let mut typedefs = HashMap::default();

        diagnostics.push(WarningDiagnostic::new(
            "Auto-inserting definition of 'va_list'",
            input.peek().source,
        ));

        typedefs.insert(
            "va_list".into(),
            CTypedef {
                ast_type: ast::Type::new(
                    ast::TypeKind::Ptr(Box::new(ast::Type::new(
                        ast::TypeKind::Void,
                        Source::internal(),
                    ))),
                    Source::internal(),
                ),
            },
        );

        Self {
            ast_file: ast::RawAstFile::new(),
            input,
            typedefs,
            enum_constants: HashMap::default(),
            diagnostics,
            c_file_type,
        }
    }

    pub fn switch_input(&mut self, tokens: Vec<CToken>) {
        self.input.switch_input(tokens);
    }

    pub fn parse(&mut self) -> Result<(), ParseError> {
        while !self.input.peek().is_end_of_file() {
            let external_declaration = self.parse_external_declaration()?;

            let mut ctx = TranslateCtx {
                ast_file: &mut self.ast_file,
                typedefs: &mut self.typedefs,
                diagnostics: self.diagnostics,
            };

            match external_declaration {
                ExternalDeclaration::Declaration(declaration) => match declaration {
                    Declaration::Common(declaration) => {
                        for init_declarator in &declaration.init_declarator_list {
                            match &init_declarator.declarator.kind {
                                DeclaratorKind::Named(..)
                                | DeclaratorKind::Pointer(..)
                                | DeclaratorKind::Array(..) => declare_named_declaration(
                                    &mut ctx,
                                    &init_declarator.declarator,
                                    &declaration.attribute_specifiers[..],
                                    &declaration.declaration_specifiers,
                                    self.c_file_type,
                                )?,
                                DeclaratorKind::Function(declarator, parameter_type_list) => {
                                    declare_function(
                                        &mut ctx,
                                        &declaration.attribute_specifiers[..],
                                        &declaration.declaration_specifiers,
                                        declarator,
                                        parameter_type_list,
                                        None,
                                        self.c_file_type,
                                    )?;
                                }
                            }
                        }
                    }
                    Declaration::StaticAssert(_) => todo!("c static assert"),
                    Declaration::Attribute(_) => todo!("c attribute declaration"),
                },
                ExternalDeclaration::FunctionDefinition(function_definition) => {
                    declare_function(
                        &mut ctx,
                        &function_definition.attributes,
                        &function_definition.declaration_specifiers,
                        &function_definition.declarator,
                        &function_definition.parameter_type_list,
                        Some(function_definition.body),
                        self.c_file_type,
                    )?;
                }
            }
        }

        Ok(())
    }

    fn parse_external_declaration(&mut self) -> Result<ExternalDeclaration, ParseError> {
        if let Ok(declaration) = speculate!(self.input, self.parse_declaration()) {
            return Ok(declaration.into());
        }

        Ok(speculate!(self.input, self.parse_function_definition())?.into())
    }

    fn parse_function_definition(&mut self) -> Result<FunctionDefinition, ParseError> {
        let attributes = self.parse_attribute_specifier_sequence()?;
        let declaration_specifiers = self.parse_declaration_specifiers()?;
        let declarator = self.parse_declarator()?;
        let body = self.parse_function_body()?;

        let (declarator, parameter_type_list) = match declarator.kind {
            DeclaratorKind::Function(declarator, parameter_type_list) => {
                (*declarator, parameter_type_list)
            }
            _ => {
                return Err(ParseError::message(
                    "Invalid function definition",
                    declarator.source,
                ));
            }
        };

        Ok(FunctionDefinition {
            attributes,
            declaration_specifiers,
            declarator,
            parameter_type_list,
            body,
        })
    }

    fn parse_attribute_specifier_sequence(&mut self) -> Result<Vec<Attribute>, ParseError> {
        #[allow(clippy::never_loop)]
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
        let source = self.here();
        let mut specifiers = vec![];

        while let Ok(specifier) = speculate!(self.input, self.parse_declaration_specifier()) {
            specifiers.push(specifier);
        }

        let attributes = self.parse_attribute_specifier_sequence()?;
        Ok(DeclarationSpecifiers {
            specifiers,
            attributes,
            source,
        })
    }

    fn parse_declaration_specifier(&mut self) -> Result<DeclarationSpecifier, ParseError> {
        let CToken { kind, source } = self.input.peek();
        let source = *source;

        let result: DeclarationSpecifierKind = match kind {
            CTokenKind::AutoKeyword => StorageClassSpecifier::Auto.into(),
            CTokenKind::ConstexprKeyword => StorageClassSpecifier::Constexpr.into(),
            CTokenKind::ExternKeyword => StorageClassSpecifier::Extern.into(),
            CTokenKind::RegisterKeyword => StorageClassSpecifier::Register.into(),
            CTokenKind::StaticKeyword => StorageClassSpecifier::Static.into(),
            CTokenKind::ThreadLocalKeyword => StorageClassSpecifier::ThreadLocal.into(),
            CTokenKind::TypedefKeyword => StorageClassSpecifier::Typedef.into(),
            CTokenKind::InlineKeyword => FunctionSpecifier::Inline.into(),
            CTokenKind::NoreturnKeyword => FunctionSpecifier::Noreturn.into(),
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

        Err(self.error("Failed to parse type specifier qualifier"))
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

        let source = self.here();

        #[allow(clippy::redundant_pattern_matching)]
        if let Ok(..) = speculate!(self.input, self.parse_atomic_type_specifier()) {
            return Ok(todo!("parse atomic type specifier"));
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

        #[allow(clippy::redundant_pattern_matching)]
        if let Ok(..) = speculate!(self.input, self.parse_typeof_specifier()) {
            return Ok(todo!("parse c typeof"));
        }

        Err(self.error("Failed to parse type specifier"))
    }

    fn parse_type_qualifier(&mut self) -> Result<TypeQualifier, ParseError> {
        let kind = match self.input.peek().kind {
            CTokenKind::ConstKeyword => TypeQualifierKind::Const,
            CTokenKind::RestrictKeyword => TypeQualifierKind::Restrict,
            CTokenKind::VolatileKeyword => TypeQualifierKind::Volatile,
            CTokenKind::AtomicKeyword => TypeQualifierKind::Atomic,
            _ => return Err(self.error("Failed to parse type qualifier")),
        };

        let source = self.input.advance().source;
        Ok(TypeQualifier { kind, source })
    }

    fn parse_alignment_specifier(&mut self) -> Result<AlignmentSpecifier, ParseError> {
        if !self.eat(CTokenKind::AlignasKeyword) {
            return Err(self.error("Expected 'alignas' keyword to begin alignment specifier"));
        }

        if !self.eat_open_paren() {
            return Err(self.error("Expected '(' after 'alignas' keyword"));
        }

        todo!("parse_alignment_specifier")
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
            return Err(self.error("Expected abstract declarator"));
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

        while let Ok(qualifier) = speculate!(self.input, self.parse_type_qualifier()) {
            qualifiers.push(qualifier);
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
                return Err(self.error("Expected ')' to close nested direct declarator"));
            }

            declarator
        } else {
            return Err(self.error("Expected declarator"));
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
                return Err(self.error("Expected ')' to close nested direct abstract declarator"));
            }

            declarator
        } else {
            AbstractDeclaratorKind::Nothing.at(self.here())
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
        let source = self.here();

        if !self.eat_punctuator(Punctuator::OpenBracket) {
            return Err(self.error("Expected '[' to begin array declarator"));
        }

        let mut is_static = self.eat(CTokenKind::StaticKeyword);
        let type_qualifiers = self.parse_type_qualifier_list()?;

        if !is_static {
            is_static = self.eat(CTokenKind::StaticKeyword);
        }

        let expression = speculate!(self.input, self.parse_expr_singular()).ok();
        let is_param_vla = expression.is_none() && self.eat_punctuator(Punctuator::Multiply);

        if !self.eat_punctuator(Punctuator::CloseBracket) {
            return Err(self.error("Expected ']' to close array declarator"));
        }

        if is_param_vla && !type_qualifiers.is_empty() && abstraction.is_abstract() {
            return Err(self.error("Cannot specify type qualifiers for abstract array declarator"));
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
        let source = self.here();
        let array_qualifier = self.parse_array_qualifier(Abstraction::Normal)?;
        Ok(DeclaratorKind::Array(Box::new(declarator), array_qualifier).at(source))
    }

    fn parse_abstract_array_declarator(
        &mut self,
        declarator: AbstractDeclarator,
    ) -> Result<AbstractDeclarator, ParseError> {
        let source = self.here();
        let array_qualifier = self.parse_array_qualifier(Abstraction::Abstract)?;
        Ok(AbstractDeclaratorKind::Array(Box::new(declarator), array_qualifier).at(source))
    }

    fn parse_function_declarator(
        &mut self,
        declarator: Declarator,
    ) -> Result<Declarator, ParseError> {
        let source = self.here();

        if !self.eat_open_paren() {
            return Err(self.error("Expected '(' to begin function declarator"));
        }

        let parameter_type_list = self.parse_parameter_type_list()?;

        if !self.eat_punctuator(Punctuator::CloseParen) {
            return Err(self.error("Expected ')' to close function declarator"));
        }

        Ok(DeclaratorKind::Function(Box::new(declarator), parameter_type_list).at(source))
    }

    fn parse_abstract_function_declarator(
        &mut self,
        declarator: AbstractDeclarator,
    ) -> Result<AbstractDeclarator, ParseError> {
        let source = self.here();

        if !self.eat_open_paren() {
            return Err(self.error("Expected '(' to begin abstract function declarator"));
        }

        let parameter_type_list = self.parse_parameter_type_list()?;

        if !self.eat_punctuator(Punctuator::CloseParen) {
            return Err(self.error("Expected ')' to close abstract function declarator"));
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
                    return Err(self
                        .error("Expected ',' after parameter declaration in parameter type list"));
                }
            }
        }

        Ok(ParameterTypeList {
            parameter_declarations,
            is_variadic,
        })
    }

    fn parse_parameter_declaration(&mut self) -> Result<ParameterDeclaration, ParseError> {
        let source = self.here();
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
            todo!("parse atomic keyword")
        }

        Err(self.error("Failed to parse atomic specifier"))
    }

    fn parse_struct_or_union_specifier(&mut self) -> Result<Composite, ParseError> {
        let kind = match self.input.peek().kind {
            CTokenKind::StructKeyword => CompositeKind::Struct,
            CTokenKind::UnionKeyword => CompositeKind::Union,
            _ => return Err(self.error("Failed to parse struct or union specifier")),
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
            return Err(ParseErrorKind::ExpectedTypeNameOrMemberDeclarationList.at(self.here()));
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
            return Err(self.error("Expected '{' to begin member declaration list"));
        }

        let mut member_declarations = vec![];

        while !matches!(
            self.input.peek().kind,
            CTokenKind::EndOfFile | CTokenKind::Punctuator(Punctuator::CloseCurly)
        ) {
            member_declarations.push(self.parse_member_declaration()?);
        }

        if !self.eat_punctuator(Punctuator::CloseCurly) {
            return Err(self.error("Expected '}' to close member declaration list"));
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
            return Err(ParseErrorKind::ExpectedSemicolon.at(self.here()));
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
            if !member_declarators.is_empty() && !self.eat_punctuator(Punctuator::Comma) {
                break;
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
            Err(ParseErrorKind::ExpectedMemberDeclarator.at(self.here()))
        }
    }

    fn parse_constant_expression(&mut self) -> Result<ConstExpr, ParseError> {
        let value = self.parse_expr_singular()?;

        // NOTE: We won't check if the constant expression is actually constant
        // unless we actually need to evaluate it
        Ok(ConstExpr { value })
    }

    fn parse_specifier_qualifier_list(&mut self) -> Result<SpecifierQualifierList, ParseError> {
        let source = self.here();
        let mut type_specifier_qualifiers = vec![];

        while let Ok(qualifier) = speculate!(self.input, self.parse_type_specifier_qualifier()) {
            type_specifier_qualifiers.push(qualifier);
        }

        let attributes = self.parse_attribute_specifier_sequence()?;

        Ok(SpecifierQualifierList {
            attributes,
            type_specifier_qualifiers,
            source,
        })
    }

    fn parse_typedef_name(&mut self) -> Result<TypedefName, ParseError> {
        if let CTokenKind::Identifier(name) = &self.input.peek().kind {
            if self.typedefs.contains_key(name) {
                let name = name.clone();
                let source = self.input.advance().source;
                return Ok(TypedefName { name, source });
            }
        }

        Err(self.error("Failed to parse typedef name"))
    }

    fn parse_enum_specifier(&mut self) -> Result<Enumeration, ParseError> {
        let source = self.here();

        if !self.eat(CTokenKind::EnumKeyword) {
            return Err(self.error("Failed to parse enum specifier"));
        }

        let enum_start_of_head = self.here();
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

            Ok(Enumeration::Named(EnumerationNamed {
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
                return Err(self.error("Expected ',' or '}' after enumerator"));
            }
        }

        if !self.eat_punctuator(Punctuator::CloseCurly) {
            return Err(self.error("Expected '}' after enumeration body"));
        }

        Ok(Some(enumerators))
    }

    fn parse_enumerator(&mut self) -> Result<Enumerator, ParseError> {
        let (name, source) = self
            .eat_identifier_source()
            .ok_or_else(|| self.error("Expected name of enumerator inside enumeration body"))?;

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
            CTokenKind::TypeofKeyword => todo!("parse c typeof keyword"),
            CTokenKind::TypeofUnqualKeyword => todo!("parse c typeof_unqual keyword"),
            _ => Err(self.error("Failed to parse typeof specifier")),
        }
    }

    fn parse_static_assert(&mut self) -> Result<StaticAssertDeclaration, ParseError> {
        if !self.eat(CTokenKind::StaticAssertKeyword) {
            return Err(self.error("Expected 'static_assert' keyword to begin static assert"));
        }

        if !self.eat_open_paren() {
            return Err(self.error("Expected '(' after 'static_assert' keyword"));
        }

        let condition = self.parse_constant_expression()?;

        let message = if self.eat_punctuator(Punctuator::Comma) {
            let token = self.input.advance();

            let CTokenKind::StringLiteral(_encoding, string) = &token.kind else {
                return Err(ParseError::message(
                    "Expected message for 'static_assert'",
                    token.source,
                ));
            };

            Some(string.clone())
        } else {
            None
        };

        if !self.eat_punctuator(Punctuator::CloseParen) {
            return Err(self.error("Expected ')' to close 'static_assert' statement"));
        }

        if !self.eat_punctuator(Punctuator::Semicolon) {
            return Err(self.error("Expected ';' after 'static_assert' statement"));
        }

        Ok(StaticAssertDeclaration { condition, message })
    }

    fn parse_declaration(&mut self) -> Result<Declaration, ParseError> {
        if self.input.peek_is(CTokenKind::StaticAssertKeyword) {
            return Ok(Declaration::StaticAssert(self.parse_static_assert()?));
        }

        let attribute_specifiers = self.parse_attribute_specifier_sequence()?;

        if !attribute_specifiers.is_empty() && self.eat_punctuator(Punctuator::Semicolon) {
            // attribute-declaration
            return Ok(todo!("parse attribute declaration"));
        }

        let declaration_specifiers = self.parse_declaration_specifiers()?;
        let init_declarator_list = self.parse_init_declarator_list()?;

        if !attribute_specifiers.is_empty() && init_declarator_list.is_empty() {
            return Err(self
                .error("Expected at least one init declarator when attribute specifiers present"));
        }

        if !self.eat_punctuator(Punctuator::Semicolon) {
            // TODO: Improve error message
            return Err(self.error("Expected ';' after declaration"));
        }

        Ok(Declaration::Common(CommonDeclaration {
            attribute_specifiers,
            declaration_specifiers,
            init_declarator_list,
        }))
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

    fn parse_initializer(&mut self) -> Result<Initializer, ParseError> {
        if let Ok(braced_initializer) = speculate!(self.input, self.parse_braced_initializer()) {
            return Ok(Initializer::BracedInitializer(braced_initializer));
        }

        if let Ok(expr) = speculate!(self.input, self.parse_expr_singular()) {
            return Ok(Initializer::Expression(expr));
        }

        Err(self.error("Expected initializer"))
    }

    fn parse_braced_initializer(&mut self) -> Result<BracedInitializer, ParseError> {
        if !self.eat_punctuator(Punctuator::OpenCurly) {
            return Err(self.error("Expected '{' to begin braced initializer"));
        }

        let mut designated_initializers = Vec::new();

        while !self
            .input
            .peek_is_or_eof(CTokenKind::Punctuator(Punctuator::CloseCurly))
        {
            designated_initializers.push(self.parse_designated_initializer()?);

            if !self.eat_punctuator(Punctuator::Comma)
                && !self
                    .input
                    .peek_is(CTokenKind::Punctuator(Punctuator::CloseCurly))
            {
                return Err(self.error("Expected ',' or '}' after designated initializer"));
            }
        }

        if !self.eat_punctuator(Punctuator::CloseCurly) {
            return Err(self.error("Expected '}' to close braced initializer"));
        }

        Ok(BracedInitializer {
            designated_initializers,
        })
    }

    fn parse_designated_initializer(&mut self) -> Result<DesignatedInitializer, ParseError> {
        let designated = matches!(
            self.input.peek().kind,
            CTokenKind::Punctuator(Punctuator::Dot | Punctuator::OpenBracket)
        );

        let designation = if designated {
            Some(self.parse_designation()?)
        } else {
            None
        };

        let initializer = self.parse_initializer()?;

        Ok(DesignatedInitializer {
            designation,
            initializer,
        })
    }

    fn parse_designation(&mut self) -> Result<Designation, ParseError> {
        let path = Vec::new();

        loop {
            if self.eat_punctuator(Punctuator::OpenBracket) {
                todo!("parse_designation subscript designator");
                continue;
            }

            if self.eat_punctuator(Punctuator::Dot) {
                todo!("parse_designation field designator");
                continue;
            }

            break;
        }

        if path.is_empty() {
            return Err(self.error("Expected designation"));
        }

        Ok(Designation { path })
    }

    fn parse_function_body(&mut self) -> Result<CompoundStatement, ParseError> {
        self.parse_compound_statement()
    }

    fn parse_compound_statement(&mut self) -> Result<CompoundStatement, ParseError> {
        if !self.eat_punctuator(Punctuator::OpenCurly) {
            return Err(self.error("Expected '{' to begin compound statement"));
        }

        let mut statements = vec![];

        loop {
            if self.eat_punctuator(Punctuator::CloseCurly) {
                break;
            }

            let source = self.here();

            match speculate!(self.input, self.parse_declaration()) {
                Ok(declaration) => {
                    statements.push(BlockItem {
                        kind: declaration.into(),
                        source,
                    });
                    continue;
                }
                Err(error) if self.input.peek().is_static_assert_keyword() => {
                    return Err(error);
                }
                _ => (),
            }

            if let Ok(unlabeled_statement) =
                speculate!(self.input, self.parse_unlabeled_statement())
            {
                statements.push(BlockItem {
                    kind: unlabeled_statement.into(),
                    source,
                });
                continue;
            }

            if let Ok(label) = speculate!(self.input, self.parse_label()) {
                statements.push(BlockItem {
                    kind: label.into(),
                    source,
                });
                continue;
            }

            return Err(ParseErrorKind::MiscGot(
                "Expected '}' to end compound statement",
                self.input.peek().kind.clone(),
            )
            .at(self.here()));
        }

        Ok(CompoundStatement { statements })
    }

    fn parse_unlabeled_statement(&mut self) -> Result<UnlabeledStatement, ParseError> {
        if let Ok(expr_statement) = speculate!(self.input, self.parse_expr_statement()) {
            return Ok(expr_statement.into());
        }

        let attributes = self.parse_attribute_specifier_sequence()?;

        if let Ok(_primary_block) = speculate!(self.input, self.parse_primary_block()) {
            return todo!("handle parsed primary block");
        }

        if let Ok(jump_statement) = speculate!(self.input, self.parse_jump_statement()) {
            return Ok(UnlabeledStatement::JumpStatement(
                attributes,
                jump_statement,
            ));
        }

        return Err(self.error("Expected unlabeled statement"));
    }

    fn parse_primary_block(&mut self) -> Result<(), ParseError> {
        if let Ok(_compound_statement) = speculate!(self.input, self.parse_compound_statement()) {
            todo!("handle parsed compound statement");
        }

        if let Ok(_selection_statement) = speculate!(self.input, self.parse_selection_statement()) {
            todo!("handle parsed seletion statement");
        }

        if let Ok(_iteration_statement) = speculate!(self.input, self.parse_iteration_statement()) {
            todo!("handle parsed iteration statement");
        }

        Err(self.error("Expected primary block"))
    }

    fn parse_selection_statement(&mut self) -> Result<(), ParseError> {
        if self.eat(CTokenKind::IfKeyword) {
            todo!("parse if");
        }

        if self.eat(CTokenKind::SwitchKeyword) {
            todo!("parse switch");
        }

        Err(self.error("Expected selection statement"))
    }

    fn parse_iteration_statement(&mut self) -> Result<(), ParseError> {
        if self.eat(CTokenKind::WhileKeyword) {
            if !self.eat_open_paren() {
                return Err(ParseError::message("", self.here()));
            }
            todo!("parse while loop");
        }

        if self.eat(CTokenKind::ForKeyword) {
            todo!("parse for loop");
        }

        if self.eat(CTokenKind::DoKeyword) {
            todo!("parse for loop");
        }

        Err(self.error("Expected iteration statement"))
    }

    fn parse_jump_statement(&mut self) -> Result<JumpStatement, ParseError> {
        if self.eat(CTokenKind::GotoKeyword) {
            let Some(label) = self.eat_identifier() else {
                return Err(self.error("Expected label to goto"));
            };

            if !self.eat_punctuator(Punctuator::Semicolon) {
                return Err(self.error("Expected ';' after goto statement"));
            }

            return Ok(JumpStatement::Goto(label));
        }

        if self.eat(CTokenKind::ContinueKeyword) {
            if !self.eat_punctuator(Punctuator::Semicolon) {
                return Err(self.error("Expected ';' after continue statement"));
            }

            return Ok(JumpStatement::Continue);
        }

        if self.eat(CTokenKind::BreakKeyword) {
            if !self.eat_punctuator(Punctuator::Semicolon) {
                return Err(self.error("Expected ';' after break statement"));
            }

            return Ok(JumpStatement::Break);
        }

        if self.eat(CTokenKind::ReturnKeyword) {
            let result = (!self.eat_punctuator(Punctuator::Semicolon))
                .then(|| self.parse_expr_multiple())
                .transpose()?;

            if !self.eat_punctuator(Punctuator::Semicolon) {
                return Err(self.error("Expected ';' after return statement"));
            }

            return Ok(JumpStatement::Return(result));
        }

        Err(self.error("Expected jump statement"))
    }

    fn parse_expr_statement(&mut self) -> Result<ExprStatement, ParseError> {
        if self
            .input
            .peek_is(CTokenKind::Punctuator(Punctuator::Semicolon))
        {
            return Ok(ExprStatement::Empty);
        }

        let attributes = self.parse_attribute_specifier_sequence()?;
        let expr = self.parse_expr_multiple()?;

        if !self.eat_punctuator(Punctuator::Semicolon) {
            return Err(self.error("Expected ';' after statement"));
        }

        return Ok(ExprStatement::Normal(attributes, expr));
    }

    fn parse_label(&mut self) -> Result<Label, ParseError> {
        let attributes = self.parse_attribute_specifier_sequence()?;

        if self.eat(CTokenKind::CaseKeyword) {
            return Ok(Label {
                attributes,
                kind: LabelKind::Case(self.parse_constant_expression()?),
            });
        }

        if self.eat(CTokenKind::DefaultKeyword) {
            return Ok(Label {
                attributes,
                kind: LabelKind::Default,
            });
        }

        if let Some(label) = self.eat_identifier() {
            return Ok(Label {
                attributes,
                kind: LabelKind::UserDefined(label),
            });
        }

        return Err(self.error("Expected label"));
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

    pub fn here(&self) -> Source {
        self.input.here()
    }

    pub fn error(&self, message: &'static str) -> ParseError {
        ParseError::message(message, self.input.here())
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

        let source = (!expected.is_empty()).then(|| self.here());

        for _ in 0..expected.len() {
            self.input.advance();
        }

        source
    }
}
