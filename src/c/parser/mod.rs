mod error;
mod input;

use std::collections::HashMap;

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
    pub declaration_specifiers: Vec<()>,
    pub core: ParameterDeclarationCore,
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

    fn parse_declaration_specifiers(&mut self) -> Result<Vec<()>, ParseError> {
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

        self.parse_attribute_specifier_sequence()?;
        Ok(vec![])
    }

    fn parse_declaration_specifier(&mut self) -> Result<(), ParseError> {
        match self.input.peek().kind {
            CTokenKind::AutoKeyword
            | CTokenKind::ConstexprKeyword
            | CTokenKind::ExternKeyword
            | CTokenKind::RegisterKeyword
            | CTokenKind::StaticKeyword
            | CTokenKind::ThreadLocalKeyword
            | CTokenKind::TypedefKeyword => {
                self.input.advance();
                Ok(())
            }
            CTokenKind::InlineKeyword | CTokenKind::NoreturnKeyword => {
                self.input.advance();
                Ok(())
            }
            _ => self.parse_type_specifier_qualifier(),
        }
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

        if let Ok(_pointer) = self.parse_pointer() {
            self.input.success();
            todo!()
        } else {
            self.input.backtrack();
        }

        self.parse_direct_declarator()
    }

    fn parse_pointer(&mut self) -> Result<(), ParseError> {
        if !self.eat_sequence(&[CTokenKind::Punctuator(Punctuator::Multiply)]) {
            return Err(ParseError::new(
                ParseErrorKind::Misc("Failed to parse pointer type"),
                None,
            ));
        }

        self.parse_attribute_specifier_sequence()?;
        self.parse_type_qualifier_list()?;

        if let Ok(_more) = self.parse_pointer() {
            todo!()
        } else {
            todo!()
        }
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
        todo!()
    }

    fn parse_function_body(&mut self) -> Result<(), ParseError> {
        todo!()
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
