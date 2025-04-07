use super::{super::error::ParseError, Parser};
use ast::{Stmt, StmtKind};
use inflow::Inflow;
use token::{Token, TokenKind};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        // return VALUE
        //          ^

        let source = self.parse_token(TokenKind::ReturnKeyword, Some("for return statement"))?;

        let return_value = (!self.input.peek_is(TokenKind::Newline))
            .then(|| self.parse_expr())
            .transpose()?;

        Ok(StmtKind::Return(return_value).at(source))
    }
}
