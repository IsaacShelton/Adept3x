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

pub struct Parser<'a, I>
where
    I: Iterator<Item = CToken> + Clone,
{
    input: Input<'a, I>,
}

impl<'a, I> Parser<'a, I>
where
    I: Iterator<Item = CToken> + Clone,
{
    pub fn new(input: Input<'a, I>) -> Self {
        Self { input }
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
        if let Ok(function_definition) = self.parse_function_definition() {
            self.input.success();
            return Ok(());
        }
        self.input.backtrack();

        self.input.speculate();
        if let Ok(declaration) = self.parse_declaration() {
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

    fn parse_attribute_specifier_sequence(&mut self) -> Result<(), ParseError> {
        while self.eat_sequence(&[
            CTokenKind::Punctuator(Punctuator::OpenBracket),
            CTokenKind::Punctuator(Punctuator::OpenBracket),
        ]) {
            // TODO: Parse attribute
            unimplemented!("parsing c attributes");
        }

        Ok(())
    }

    fn parse_declaration_specifiers(&mut self) -> Result<(), ParseError> {
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
        Ok(())
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
        Err(todo!())
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

        Err(todo!())
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
            _ => Err(todo!()),
        }
    }

    fn parse_alignment_specifier(&mut self) -> Result<(), ParseError> {
        if !self.eat_sequence(&[CTokenKind::AlignasKeyword]) {
            return Ok(());
        }

        if let CTokenKind::Punctuator(Punctuator::OpenParen { .. }) = self.input.peek().kind {
            self.input.advance();
        } else {
            return Err(todo!());
        }

        unimplemented!("parse alignment specifier");
    }

    fn parse_declarator(&mut self) -> Result<(), ParseError> {
        self.input.speculate();

        if let Ok(pointer) = self.parse_pointer() {
            self.input.success();
            todo!()
        } else {
            self.input.backtrack();
        }

        self.parse_direct_declarator()
    }

    fn parse_pointer(&mut self) -> Result<(), ParseError> {
        if !self.eat_sequence(&[CTokenKind::Punctuator(Punctuator::Multiply)]) {
            return Err(todo!());
        }

        self.parse_attribute_specifier_sequence()?;
        self.parse_type_qualifier_list()?;

        if let Ok(more) = self.parse_pointer() {
            todo!()
        } else {
            todo!()
        }
    }

    fn parse_type_qualifier_list(&mut self) -> Result<(), ParseError> {
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

        todo!() // Ok(qualifiers)
    }

    fn parse_direct_declarator(&mut self) -> Result<(), ParseError> {
        // One of
        // identifier attribute-specifier-sequence?
        // (direct-delcarator)

        // Followed by
        // Any number of (array-declarator and function-declarator postfixes) +
        // attribute-specifier-sequence?
        todo!()
    }

    fn parse_atomic_type_specifier(&mut self) -> Result<(), ParseError> {
        todo!()
    }

    fn parse_struct_or_union_specifier(&mut self) -> Result<(), ParseError> {
        todo!()
    }

    fn parse_typedef_name(&mut self) -> Result<(), ParseError> {
        todo!()
    }

    fn parse_enum_specifier(&mut self) -> Result<(), ParseError> {
        todo!()
    }

    fn parse_typeof_specifier(&mut self) -> Result<(), ParseError> {
        todo!()
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
