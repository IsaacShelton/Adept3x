mod array_access;
mod is_match;
mod member;

use super::Parser;
use crate::error::ParseError;
use ast::Expr;
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_expr_primary_post(&mut self, mut base: Expr) -> Result<Expr, ParseError> {
        let mut ate_newline;

        loop {
            ate_newline = self
                .input
                .peek()
                .is_newline()
                .then(|| self.input.peek().clone());
            self.ignore_newlines();

            match self.input.peek().kind {
                TokenKind::Member => base = self.parse_member(base)?,
                TokenKind::OpenBracket => base = self.parse_array_access(base)?,
                TokenKind::IsKeyword => base = self.parse_is_match(base)?,
                _ => break,
            }
        }

        if let Some(newline) = ate_newline {
            self.input.unadvance(newline);
        }

        Ok(base)
    }
}
