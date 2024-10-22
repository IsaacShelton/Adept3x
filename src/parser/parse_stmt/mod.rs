mod parse_assignment;
mod parse_declaration;
mod parse_return;

use super::{
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    ast::{Stmt, StmtKind},
    inflow::Inflow,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        let stmt = self.parse_stmt_inner()?;

        if !matches!(
            self.input.peek().kind,
            TokenKind::Newline | TokenKind::CloseCurly,
        ) {
            return Err(ParseErrorKind::Other {
                message: "Expected newline or '}' after statement".into(),
            }
            .at(self.input.peek().source));
        }

        Ok(stmt)
    }

    fn parse_stmt_inner(&mut self) -> Result<Stmt, ParseError> {
        let source = self.source_here();

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
