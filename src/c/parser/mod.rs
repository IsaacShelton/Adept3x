mod error;
mod input;

pub use self::{error::ParseError, input::Input};
use super::token::CToken;
use crate::{
    ast::{Ast, File, FileIdentifier},
    source_file_cache::{SourceFileCache, SourceFileCacheKey},
};

pub struct Parser<'a, I>
where
    I: Iterator<Item = CToken>,
{
    input: Input<'a, I>,
}

impl<'a, I> Parser<'a, I>
where
    I: Iterator<Item = CToken>,
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
            self.parse_top_level(ast_file)?;
        }

        Ok(())
    }

    fn parse_top_level(&mut self, _ast_file: &mut File) -> Result<(), ParseError> {
        unimplemented!("parse c file")
    }
}

pub fn parse(
    tokens: impl Iterator<Item = CToken>,
    source_file_cache: &SourceFileCache,
    key: SourceFileCacheKey,
) -> Result<Ast, ParseError> {
    Parser::new(Input::new(tokens, source_file_cache, key)).parse()
}

pub fn parse_into(
    tokens: impl Iterator<Item = CToken>,
    source_file_cache: &SourceFileCache,
    key: SourceFileCacheKey,
    ast: &mut Ast,
    filename: String,
) -> Result<(), ParseError> {
    Parser::new(Input::new(tokens, source_file_cache, key)).parse_into(ast, filename)
}
