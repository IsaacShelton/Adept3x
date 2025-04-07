use super::{super::error::ParseError, Parser};
use ast::{Declaration, Stmt, StmtKind};
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_declaration(&mut self) -> Result<Stmt, ParseError> {
        let (name, source) = self
            .parse_identifier_keep_location(Some("for variable name"))?
            .tuple();

        let variable_type = self.parse_type(None::<&str>, Some("for variable"))?;

        let initial_value = self
            .input
            .eat(TokenKind::Assign)
            .then(|| self.parse_expr())
            .transpose()?;

        Ok(Stmt::new(
            StmtKind::Declaration(Box::new(Declaration {
                name,
                ast_type: variable_type,
                initial_value,
            })),
            source,
        ))
    }
}
