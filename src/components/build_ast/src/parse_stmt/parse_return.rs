use super::{super::error::ParseError, Parser};
use ast::{Stmt, StmtKind};
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        // return VALUE
        //          ^

        let source = self.input.here();

        self.input
            .expect(TokenKind::ReturnKeyword, "for return statement")?;

        let return_value = (!self.input.peek_is(TokenKind::Newline))
            .then(|| self.parse_expr())
            .transpose()?;

        Ok(StmtKind::Return(return_value).at(source))
    }
}
