mod error;
mod input;

use self::error::ParseErrorKind;
pub use self::{error::ParseError, input::Input};
use super::{
    punctuator::Punctuator,
    token::{CToken, CTokenKind},
};
use crate::{
    ast::{Ast, File, FileIdentifier},
    source_file_cache::{SourceFileCache, SourceFileCacheKey},
};
use std::collections::HashMap;

pub struct Parser<'a, I>
where
    I: Iterator<Item = CToken> + Clone,
{
    input: Input<'a, I>,
    typedefs: HashMap<String, CTypedef>,
}

#[derive(Clone, Debug)]
struct CTypedef {}

#[derive(Clone, Debug)]
struct ParameterTypeList {
    pub parameter_declarations: Vec<ParameterDeclaration>,
    pub is_variadic: bool,
}

#[derive(Clone, Debug)]
enum Declarator {
    Named(String),
    Pointer(Box<Declarator>),
    Function(Box<Declarator>, ParameterTypeList),
}

#[derive(Clone, Debug)]
enum AbstractDeclarator {}

#[derive(Clone, Debug)]
enum ParameterDeclarationCore {
    Declarator(Declarator),
    AbstractDeclarator(AbstractDeclarator),
    Nothing,
}

#[derive(Clone, Debug)]
struct ParameterDeclaration {
    pub attributes: Vec<()>,
    pub declaration_specifiers: DeclarationSpecifiers,
    pub core: ParameterDeclarationCore,
}

#[derive(Clone, Debug)]
struct Pointers {
    pub pointers: Vec<()>,
}

#[derive(Clone, Debug)]
struct InitDeclarator {
    pub declarator: Declarator,
    pub initializer: Option<()>,
}

#[derive(Clone, Debug)]
enum DeclarationSpecifier {
    Auto,
    Constexpr,
    Extern,
    Register,
    Static,
    ThreadLocal,
    Typedef,
    Inline,
    Noreturn,
    TypeSpecifierQualifier(()),
}

#[derive(Clone, Debug)]
struct DeclarationSpecifiers {
    pub specifiers: Vec<DeclarationSpecifier>,
    pub attributes: Vec<()>,
}

impl<'a, I> Parser<'a, I>
where
    I: Iterator<Item = CToken> + Clone,
{
    pub fn new(input: Input<'a, I>) -> Self {
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
            self.parse_external_declaration(ast_file)?;
        }

        Ok(())
    }

    fn parse_external_declaration(&mut self, _ast_file: &mut File) -> Result<(), ParseError> {
        self.input.speculate();
        if let Ok(_function_definition) = self.parse_function_definition() {
            self.input.success();
            return Ok(());
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(_declaration) = self.parse_declaration() {
            self.input.success();
            return Ok(());
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
                return Ok(DeclarationSpecifier::TypeSpecifierQualifier(
                    self.parse_type_specifier_qualifier()?,
                ));
            }
        };

        self.input.advance();
        Ok(result)
    }

    fn parse_type_specifier_qualifier(&mut self) -> Result<(), ParseError> {
        self.input.speculate();
        if let Ok(_) = self.parse_type_specifier() {
            self.input.success();
            return Ok(());
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(_) = self.parse_type_qualifier() {
            self.input.success();
            return Ok(());
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(_) = self.parse_alignment_specifier() {
            self.input.success();
            return Ok(());
        }

        self.input.backtrack();
        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse type specifier qualifier"),
            None,
        ))
    }

    fn parse_type_specifier(&mut self) -> Result<(), ParseError> {
        match self.input.peek().kind {
            CTokenKind::Decimal32Keyword => unimplemented!("_Decimal32"),
            CTokenKind::Decimal64Keyword => unimplemented!("_Decimal64"),
            CTokenKind::Decimal128Keyword => unimplemented!("_Decimal128"),
            CTokenKind::ComplexKeyword => unimplemented!("_Complex"),
            CTokenKind::BitIntKeyword => unimplemented!("_BitInt"),
            CTokenKind::VoidKeyword
            | CTokenKind::CharKeyword
            | CTokenKind::ShortKeyword
            | CTokenKind::IntKeyword
            | CTokenKind::LongKeyword
            | CTokenKind::FloatKeyword
            | CTokenKind::DoubleKeyword
            | CTokenKind::SignedKeyword
            | CTokenKind::UnsignedKeyword => {
                self.input.advance();
                return Ok(());
            }
            _ => (),
        }

        self.input.speculate();
        if let Ok(..) = self.parse_atomic_type_specifier() {
            self.input.success();
            return Ok(());
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(..) = self.parse_struct_or_union_specifier() {
            self.input.success();
            return Ok(());
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(..) = self.parse_enum_specifier() {
            self.input.success();
            return Ok(());
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(..) = self.parse_typedef_name() {
            self.input.success();
            return Ok(());
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(..) = self.parse_typeof_specifier() {
            self.input.success();
            return Ok(());
        }
        self.input.backtrack();

        Err(ParseError::new(
            ParseErrorKind::Misc("Failed to parse type specifier"),
            None,
        ))
    }

    fn parse_type_qualifier(&mut self) -> Result<(), ParseError> {
        match self.input.peek().kind {
            CTokenKind::ConstKeyword
            | CTokenKind::RestrictKeyword
            | CTokenKind::VolatileKeyword
            | CTokenKind::AtomicKeyword => {
                self.input.advance();
                Ok(())
            }
            _ => Err(ParseError::new(
                ParseErrorKind::Misc("Failed to parse type qualifier"),
                None,
            )),
        }
    }

    fn parse_alignment_specifier(&mut self) -> Result<(), ParseError> {
        self.input.speculate();
        if self.eat_sequence(&[CTokenKind::AlignasKeyword]) {
            if let CTokenKind::Punctuator(Punctuator::OpenParen { .. }) = self.input.peek().kind {
                self.input.advance();
                todo!();
                self.input.success();
                return Ok(());
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

        self.parse_direct_declarator()
    }

    fn parse_pointers(&mut self) -> Result<Pointers, ParseError> {
        let mut pointers = Pointers { pointers: vec![] };

        while self.eat_sequence(&[CTokenKind::Punctuator(Punctuator::Multiply)]) {
            self.parse_attribute_specifier_sequence()?;
            self.parse_type_qualifier_list()?;
            pointers.pointers.push(());
        }

        Ok(pointers)
    }

    fn parse_type_qualifier_list(&mut self) -> Result<Vec<()>, ParseError> {
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

    fn parse_declaration(&mut self) -> Result<(), ParseError> {
        if self.input.peek_is(CTokenKind::StaticAssertKeyword) {
            // static_assert
            todo!();
            return Ok(());
        }

        let attribute_specifiers = self.parse_attribute_specifier_sequence()?;

        if self
            .input
            .peek_is(CTokenKind::Punctuator(Punctuator::Semicolon))
        {
            // attribute-declaration
            todo!();
            return Ok(());
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

        println!("parsed declaration");
        println!(" -> attribute_specifiers = {:?}", attribute_specifiers);
        println!(" -> declaration specifiers = {:?}", declaration_specifiers);
        println!(" -> init_declarator_list = {:?}", init_declarator_list);
        return Ok(());
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
        for (i, expected_kind) in expected.iter().enumerate() {
            if self.input.peek_nth(i).kind != *expected_kind {
                return false;
            }
        }

        for _ in 0..expected.len() {
            self.input.advance();
        }

        true
    }
}

pub fn parse(
    tokens: impl Iterator<Item = CToken> + Clone,
    source_file_cache: &SourceFileCache,
    key: SourceFileCacheKey,
) -> Result<Ast, ParseError> {
    Parser::new(Input::new(tokens, source_file_cache, key)).parse()
}

pub fn parse_into(
    tokens: impl Iterator<Item = CToken> + Clone,
    source_file_cache: &SourceFileCache,
    key: SourceFileCacheKey,
    ast: &mut Ast,
    filename: String,
) -> Result<(), ParseError> {
    Parser::new(Input::new(tokens, source_file_cache, key)).parse_into(ast, filename)
}
