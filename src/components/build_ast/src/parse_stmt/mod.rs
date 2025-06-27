mod parse_assignment;
mod parse_declaration;
mod parse_goto;
mod parse_return;

use super::{
    Parser,
    error::{ParseError, ParseErrorKind},
};
use ast::{Stmt, StmtKind};
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        let stmt = self.parse_stmt_inner()?;

        if !matches!(
            self.input.peek().kind,
            TokenKind::Newline | TokenKind::CloseCurly,
        ) {
            return Err(ParseErrorKind::Expected {
                expected: "newline or '}' after statement".into(),
                for_reason: None,
                got: self.input.peek().to_string(),
            }
            .at(self.input.peek().source));
        }

        Ok(stmt)
    }

    fn parse_stmt_inner(&mut self) -> Result<Stmt, ParseError> {
        let source = self.input.here();

        match self.input.peek().kind {
            TokenKind::Identifier(_) => {
                if !self.input.peek().is_assignment_like()
                    && self.input.peek_nth(1).kind.could_start_type()
                {
                    // Explicitly-Typed Variable Declaration Statement
                    return self.parse_declaration();
                }

                ()
            }
            TokenKind::ReturnKeyword => return self.parse_return(),
            TokenKind::GotoKeyword => return self.parse_goto(),
            TokenKind::Label(_) => {
                let name = self.input.advance().kind.unwrap_label();
                return Ok(StmtKind::Label(name).at(source));
            }
            TokenKind::EndOfFile => return Err(self.unexpected_token_is_next()),
            _ => (),
        }

        let lhs = self.parse_expr_primary()?;

        if self.input.peek().is_assignment_like() {
            // Assignment-Like Statement
            return self.parse_assignment(lhs);
        }

        let lhs = self.parse_operator_expr(0, lhs)?;

        // Plain Expression Statement
        Ok(StmtKind::Expr(lhs).at(source))
    }
}
