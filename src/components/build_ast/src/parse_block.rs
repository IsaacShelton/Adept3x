use super::{Parser, error::ParseError};
use ast::Stmt;
use infinite_iterator::InfinitePeekable;
use lazy_format::lazy_format;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_block(&mut self, to_begin_what_block: &str) -> Result<Vec<Stmt>, ParseError> {
        self.ignore_newlines();

        self.parse_token(
            TokenKind::OpenCurly,
            Some(lazy_format!("to begin {} block", to_begin_what_block)),
        )?;

        let mut stmts = Vec::new();
        self.ignore_newlines();

        while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
            stmts.push(self.parse_stmt()?);
            self.ignore_newlines();
        }

        self.ignore_newlines();
        self.parse_token(
            TokenKind::CloseCurly,
            Some(lazy_format!("to close {} block", to_begin_what_block)),
        )?;

        Ok(stmts)
    }
}
