use super::Parser;
use crate::error::ParseError;
use ast::{ArrayAccess, Expr, ExprKind};
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_array_access(&mut self, subject: Expr) -> Result<Expr, ParseError> {
        // subject[index]
        //        ^

        let source = self.input.here();

        self.input
            .expect(TokenKind::OpenBracket, "for array access")?;

        self.ignore_newlines();
        let index = self.parse_expr()?;
        self.ignore_newlines();

        self.input
            .expect(TokenKind::CloseBracket, "to close array access")?;

        Ok(ExprKind::ArrayAccess(Box::new(ArrayAccess { subject, index })).at(source))
    }
}
