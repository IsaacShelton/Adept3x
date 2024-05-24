mod error;
mod input;
mod translation;

use itertools::Itertools;

use self::error::ParseErrorKind;
pub use self::{error::ParseError, input::Input};
use super::{
    punctuator::Punctuator,
    token::{CToken, CTokenKind},
};
use crate::{
    ast::{Ast, FileIdentifier, Source},
    c::parser::translation::declare_function,
    source_file_cache::{SourceFileCache, SourceFileCacheKey},
};
use std::collections::HashMap;

pub struct Parser<'a> {
    input: Input<'a>,
    typedefs: HashMap<String, CTypedef>,
}

#[derive(Clone, Debug)]
pub struct CTypedef {}

#[derive(Clone, Debug)]
pub struct ParameterTypeList {
    pub parameter_declarations: Vec<ParameterDeclaration>,
    pub is_variadic: bool,
}

#[derive(Clone, Debug)]
pub enum Declarator {
    Named(String),
    Pointers(Box<Declarator>, Pointers),
    Function(Box<Declarator>, ParameterTypeList),
}

#[derive(Clone, Debug)]
pub enum AbstractDeclarator {}

#[derive(Clone, Debug)]
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
}

#[derive(Clone, Debug, Default)]
pub struct Pointers {
    pub pointers: Vec<Pointer>,
}

impl Pointers {
    pub fn concat(&self, other: &Pointers) -> Self {
        let pointers = self
            .pointers
            .iter()
            .chain(other.pointers.iter())
            .cloned()
            .collect_vec();

        Self { pointers }
    }
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
pub enum DeclarationSpecifier {
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

#[derive(Clone, Debug)]
pub enum TypeSpecifierQualifier {
    TypeSpecifier(TypeSpecifier),
    TypeQualifier(TypeQualifier),
    AlignmentSpecifier(AlignmentSpecifier),
}

#[derive(Clone, Debug)]
pub struct TypeSpecifier {
    kind: TypeSpecifierKind,
    source: Source,
}

#[derive(Clone, Debug)]
pub enum TypeSpecifierKind {
    Void,
    Char,
    Short,
    Int,
    Long,
    Float,
    Double,
    Signed,
    Unsigned,
}

#[derive(Clone, Debug)]
pub enum TypeQualifier {
    Const,
    Restrict,
    Volatile,
    Atomic,
}

#[derive(Clone, Debug)]
pub enum AlignmentSpecifier {
    AlignAsType(()),
    AlisnAsConstExpr(()),
}

#[derive(Clone, Debug)]
pub struct DeclarationSpecifiers {
    pub specifiers: Vec<DeclarationSpecifier>,
    pub attributes: Vec<()>,
}

#[derive(Clone, Debug)]
pub struct ExternalDeclaration {
    pub attribute_specifiers: Vec<()>,
    pub declaration_specifiers: DeclarationSpecifiers,
    pub init_declarator_list: Vec<InitDeclarator>,
}

impl<'a> Parser<'a> {
    pub fn new(input: Input<'a>) -> Self {
        Self {
            input,
            typedefs: HashMap::default(),
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

            for init_declarator in external_declaration.init_declarator_list.iter() {
                match &init_declarator.declarator {
                    Declarator::Named(_) => todo!(),
                    Declarator::Pointers(_, _) => todo!(),
                    Declarator::Function(declarator, parameter_type_list) => {
                        declare_function(
                            ast_file,
                            &external_declaration.attribute_specifiers[..],
                            &external_declaration.declaration_specifiers,
                            declarator,
                            parameter_type_list,
                        )?;
                    }
                }
            }
        }

        Ok(())
    }

    fn parse_external_declaration(&mut self) -> Result<ExternalDeclaration, ParseError> {
        self.input.speculate();
        if let Ok(_function_definition) = self.parse_function_definition() {
            self.input.success();
            return Ok(todo!());
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(declaration) = self.parse_declaration() {
            self.input.success();
            return Ok(declaration);
        }
        self.input.backtrack();

        Err(ParseError::new(ParseErrorKind::ExpectedDeclaration, None))
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
            self.input.speculate();

            if let Ok(specifier) = self.parse_declaration_specifier() {
                self.input.success();
                specifiers.push(specifier);
                continue;
            }

            self.input.backtrack();
            break;
        }

        let attributes = self.parse_attribute_specifier_sequence()?;
        Ok(DeclarationSpecifiers {
            specifiers,
            attributes,
        })
    }

    fn parse_declaration_specifier(&mut self) -> Result<DeclarationSpecifier, ParseError> {
        let result = match self.input.peek().kind {
            CTokenKind::AutoKeyword => DeclarationSpecifier::Auto,
            CTokenKind::ConstexprKeyword => DeclarationSpecifier::Constexpr,
            CTokenKind::ExternKeyword => DeclarationSpecifier::Extern,
            CTokenKind::RegisterKeyword => DeclarationSpecifier::Register,
            CTokenKind::StaticKeyword => DeclarationSpecifier::Static,
            CTokenKind::ThreadLocalKeyword => DeclarationSpecifier::ThreadLocal,
            CTokenKind::TypedefKeyword => DeclarationSpecifier::Typedef,
            CTokenKind::InlineKeyword => DeclarationSpecifier::Inline,
            CTokenKind::NoreturnKeyword => DeclarationSpecifier::Noreturn,
            _ => {
                let r = DeclarationSpecifier::TypeSpecifierQualifier(
                    self.parse_type_specifier_qualifier()?,
                );
                return Ok(r);
            }
        };

        self.input.advance();
        Ok(result)
    }

    fn parse_type_specifier_qualifier(&mut self) -> Result<TypeSpecifierQualifier, ParseError> {
        self.input.speculate();
        if let Ok(type_specifier) = self.parse_type_specifier() {
            self.input.success();
            return Ok(TypeSpecifierQualifier::TypeSpecifier(type_specifier));
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(type_qualifier) = self.parse_type_qualifier() {
            self.input.success();
            return Ok(TypeSpecifierQualifier::TypeQualifier(type_qualifier));
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(alignment_specifier) = self.parse_alignment_specifier() {
            self.input.success();
            return Ok(TypeSpecifierQualifier::AlignmentSpecifier(
                alignment_specifier,
            ));
        }

        self.input.backtrack();
        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse type specifier qualifier"),
            None,
        ))
    }

    fn parse_type_specifier(&mut self) -> Result<TypeSpecifier, ParseError> {
        if let Some(type_specifier_kind) = match self.input.peek().kind {
            CTokenKind::Decimal32Keyword => unimplemented!("_Decimal32"),
            CTokenKind::Decimal64Keyword => unimplemented!("_Decimal64"),
            CTokenKind::Decimal128Keyword => unimplemented!("_Decimal128"),
            CTokenKind::ComplexKeyword => unimplemented!("_Complex"),
            CTokenKind::BitIntKeyword => unimplemented!("_BitInt"),
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

        self.input.speculate();
        if let Ok(..) = self.parse_atomic_type_specifier() {
            self.input.success();
            return Ok(todo!());
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(..) = self.parse_struct_or_union_specifier() {
            self.input.success();
            return Ok(todo!());
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(..) = self.parse_enum_specifier() {
            self.input.success();
            return Ok(todo!());
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(..) = self.parse_typedef_name() {
            self.input.success();
            return Ok(todo!());
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(..) = self.parse_typeof_specifier() {
            self.input.success();
            return Ok(todo!());
        }
        self.input.backtrack();

        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse type specifier"),
            None,
        ))
    }

    fn parse_type_qualifier(&mut self) -> Result<TypeQualifier, ParseError> {
        let type_qualifier = match self.input.peek().kind {
            CTokenKind::ConstKeyword => TypeQualifier::Const,
            CTokenKind::RestrictKeyword => TypeQualifier::Restrict,
            CTokenKind::VolatileKeyword => TypeQualifier::Volatile,
            CTokenKind::AtomicKeyword => TypeQualifier::Atomic,
            _ => {
                return Err(ParseError::new(
                    ParseErrorKind::Misc("Failed to parse type qualifier"),
                    None,
                ))
            }
        };

        self.input.advance();
        Ok(type_qualifier)
    }

    fn parse_alignment_specifier(&mut self) -> Result<AlignmentSpecifier, ParseError> {
        self.input.speculate();
        if self.eat_sequence(&[CTokenKind::AlignasKeyword]) {
            if let CTokenKind::Punctuator(Punctuator::OpenParen { .. }) = self.input.peek().kind {
                self.input.advance();
                todo!();
                self.input.success();
                return Ok(todo!());
            }
        }

        self.input.backtrack();

        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse alignment specifier"),
            None,
        ))
    }

    fn parse_declarator(&mut self) -> Result<Declarator, ParseError> {
        self.input.speculate();

        let pointers = match self.parse_pointers() {
            Ok(pointers) => {
                self.input.success();
                pointers
            }
            Err(err) => {
                self.input.backtrack();
                return Err(err);
            }
        };

        let declarator = self.parse_direct_declarator()?;

        if pointers.pointers.is_empty() {
            Ok(declarator)
        } else {
            Ok(Declarator::Pointers(Box::new(declarator), pointers))
        }
    }

    fn parse_pointers(&mut self) -> Result<Pointers, ParseError> {
        let mut pointers = Pointers { pointers: vec![] };

        while let Some(source) =
            self.eat_sequence_source(&[CTokenKind::Punctuator(Punctuator::Multiply)])
        {
            let attributes = self.parse_attribute_specifier_sequence()?;

            let type_qualifiers = self.parse_type_qualifier_list()?;
            pointers.pointers.push(Pointer {
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
            self.input.speculate();

            if let Ok(qualifier) = self.parse_type_qualifier() {
                self.input.success();
                qualifiers.push(qualifier);
                continue;
            }

            self.input.backtrack();
            break;
        }

        Ok(qualifiers)
    }

    fn parse_direct_declarator(&mut self) -> Result<Declarator, ParseError> {
        let mut declarator = if let CTokenKind::Identifier(name) = &self.input.peek().kind {
            let name = name.clone();
            self.input.advance();
            let attributes = self.parse_attribute_specifier_sequence()?;
            Declarator::Named(name)
        } else if let CTokenKind::Punctuator(Punctuator::OpenParen { .. }) = &self.input.peek().kind
        {
            self.input.advance();
            let declarator = self.parse_declarator()?;

            if !self.eat_sequence(&[CTokenKind::Punctuator(Punctuator::CloseParen)]) {
                return Err(ParseError::new(
                    ParseErrorKind::Misc("Failed to parse ')' for direct declarator"),
                    None,
                ));
            }

            declarator
        } else {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected declarator"),
                None,
            ));
        };

        loop {
            match &self.input.peek().kind {
                CTokenKind::Punctuator(Punctuator::OpenBracket) => todo!(),
                CTokenKind::Punctuator(Punctuator::OpenParen { .. }) => {
                    declarator = self.parse_function_declarator(declarator)?
                }
                _ => break,
            }
        }

        let attributes = self.parse_attribute_specifier_sequence()?;
        Ok(declarator)
    }

    fn parse_function_declarator(
        &mut self,
        declarator: Declarator,
    ) -> Result<Declarator, ParseError> {
        assert!(self.input.advance().kind.is_open_paren());
        let parameter_type_list = self.parse_parameter_type_list()?;
        self.eat_sequence(&[CTokenKind::Punctuator(Punctuator::CloseParen)]);
        Ok(Declarator::Function(
            Box::new(declarator),
            parameter_type_list,
        ))
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
                        None,
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
        let attributes = self.parse_attribute_specifier_sequence()?;
        let declaration_specifiers = self.parse_declaration_specifiers()?;

        self.input.speculate();
        if let Ok(declarator) = self.parse_declarator() {
            self.input.success();
            return Ok(ParameterDeclaration {
                attributes,
                declaration_specifiers,
                core: ParameterDeclarationCore::Declarator(declarator),
            });
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(abstract_declarator) = self.parse_abstract_declarator() {
            self.input.success();
            return Ok(ParameterDeclaration {
                attributes,
                declaration_specifiers,
                core: ParameterDeclarationCore::AbstractDeclarator(abstract_declarator),
            });
        }
        self.input.backtrack();

        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse parameter declaration"),
            None,
        ))
    }

    fn parse_abstract_declarator(&mut self) -> Result<AbstractDeclarator, ParseError> {
        todo!()
    }

    fn parse_atomic_type_specifier(&mut self) -> Result<(), ParseError> {
        if self.input.peek_is(CTokenKind::AtomicKeyword) {
            todo!()
        }

        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse atomic specifier"),
            None,
        ))
    }

    fn parse_struct_or_union_specifier(&mut self) -> Result<(), ParseError> {
        if let CTokenKind::StructKeyword | CTokenKind::UnionKeyword = self.input.peek().kind {
            self.input.advance();
            todo!()
        }

        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse struct or union specifier"),
            None,
        ))
    }

    fn parse_typedef_name(&mut self) -> Result<(), ParseError> {
        if let CTokenKind::Identifier(name) = &self.input.peek().kind {
            if let Some(_typedef) = self.typedefs.get(name) {
                self.input.advance();
                return Ok(());
            }
        }

        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse typedef name"),
            None,
        ))
    }

    fn parse_enum_specifier(&mut self) -> Result<(), ParseError> {
        if let CTokenKind::StructKeyword | CTokenKind::UnionKeyword = self.input.peek().kind {
            self.input.advance();
            todo!();
        }

        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse enum specifier"),
            None,
        ))
    }

    fn parse_typeof_specifier(&mut self) -> Result<(), ParseError> {
        self.input.speculate();

        if let CTokenKind::TypeofKeyword | CTokenKind::TypeofUnqualKeyword =
            self.input.advance().kind
        {
            todo!()
        }

        self.input.backtrack();
        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse typeof specifier"),
            None,
        ))
    }

    fn parse_declaration(&mut self) -> Result<ExternalDeclaration, ParseError> {
        if self.input.peek_is(CTokenKind::StaticAssertKeyword) {
            // static_assert
            todo!();
            return Ok(todo!());
        }

        let attribute_specifiers = self.parse_attribute_specifier_sequence()?;

        if self
            .input
            .peek_is(CTokenKind::Punctuator(Punctuator::Semicolon))
        {
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
                None,
            ));
        }

        if !self.eat_sequence(&[CTokenKind::Punctuator(Punctuator::Semicolon)]) {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected ';' after declaration"),
                None,
            ));
        }

        return Ok(ExternalDeclaration {
            attribute_specifiers,
            declaration_specifiers,
            init_declarator_list,
        });
    }

    fn parse_init_declarator_list(&mut self) -> Result<Vec<InitDeclarator>, ParseError> {
        let mut list = vec![];

        loop {
            self.input.speculate();
            if let Ok(init_declarator) = self.parse_init_declarator() {
                list.push(init_declarator);
                self.input.success();
            } else {
                self.input.backtrack();
                return Ok(list);
            };

            match self.input.peek().kind {
                CTokenKind::Punctuator(Punctuator::Comma) => (),
                _ => break,
            }
        }

        Ok(list)
    }

    fn parse_init_declarator(&mut self) -> Result<InitDeclarator, ParseError> {
        let declarator = self.parse_declarator()?;

        let initializer = if self.eat_sequence(&[CTokenKind::Punctuator(Punctuator::Assign)]) {
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
        self.input.speculate();
        if let Ok(..) = self.parse_braced_initializer() {
            self.input.success();
            todo!();
            return Ok(());
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(..) = self.parse_assignment_expression() {
            self.input.success();
            todo!();
            return Ok(());
        }
        self.input.backtrack();

        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse initializer"),
            None,
        ))
    }

    fn parse_braced_initializer(&mut self) -> Result<(), ParseError> {
        if !self.eat_sequence(&[CTokenKind::Punctuator(Punctuator::OpenCurly)]) {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected '{' to begin braced initializer"),
                None,
            ));
        }

        todo!();

        if !self.eat_sequence(&[CTokenKind::Punctuator(Punctuator::CloseCurly)]) {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected '}' to close braced initializer"),
                None,
            ));
        }

        todo!();
        Ok(())
    }

    fn parse_assignment_expression(&mut self) -> Result<(), ParseError> {
        todo!()
    }

    fn parse_function_body(&mut self) -> Result<(), ParseError> {
        self.parse_compound_statement()
    }

    fn parse_compound_statement(&mut self) -> Result<(), ParseError> {
        if !self.eat_sequence(&[CTokenKind::Punctuator(Punctuator::OpenCurly)]) {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected '{' to begin compound statement"),
                None,
            ));
        }

        todo!();

        if !self.eat_sequence(&[CTokenKind::Punctuator(Punctuator::CloseCurly)]) {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Expected '}' to close compound statement"),
                None,
            ));
        }

        Ok(())
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
