mod array_access;
mod member;

use super::Parser;
use crate::{
    ast::Expr,
    inflow::Inflow,
    parser::error::ParseError,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_expr_primary_post(&mut self, mut base: Expr) -> Result<Expr, ParseError> {
        loop {
            self.ignore_newlines();

            match self.input.peek().kind {
                TokenKind::Member => base = self.parse_member(base)?,
                TokenKind::OpenBracket => base = self.parse_array_access(base)?,
                _ => break,
            }
        }

        Ok(base)
    }
}
