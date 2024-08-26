mod parse_assignment;
mod parse_declaration;
mod parse_return;

use super::{error::ParseError, Parser};
use crate::{
    ast::{Stmt, StmtKind},
    inflow::Inflow,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        let source = self.source_here();

        match self.input.peek().kind {
            TokenKind::Identifier(_) => {
                if !self.input.peek().is_assignment_like()
                    && self.input.peek_nth(1).kind.could_start_type()
                {
                    // Explicitly-Typed Variable Declaration Statement
                    return self.parse_declaration();
                }

                let lhs = self.parse_expr()?;

                if self.input.peek().is_assignment_like() {
                    // Assignment-Like Statement
                    return self.parse_assignment(lhs);
                }

                // Plain Expression Statement
                Ok(StmtKind::Expr(lhs).at(source))
            }
            TokenKind::ReturnKeyword => self.parse_return(),
            TokenKind::EndOfFile => Err(self.unexpected_token_is_next()),
            _ => Ok(Stmt::new(StmtKind::Expr(self.parse_expr()?), source)),
        }
    }
}
