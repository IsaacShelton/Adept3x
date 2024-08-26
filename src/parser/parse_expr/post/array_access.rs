use super::Parser;
use crate::{
    ast::{ArrayAccess, Expr, ExprKind},
    inflow::Inflow,
    parser::error::ParseError,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_array_access(&mut self, subject: Expr) -> Result<Expr, ParseError> {
        // subject[index]
        //        ^

        let source = self.parse_token(TokenKind::OpenBracket, Some("for array access"))?;

        self.ignore_newlines();
        let index = self.parse_expr()?;
        self.ignore_newlines();

        self.parse_token(TokenKind::CloseBracket, Some("to close array access"))?;

        Ok(ExprKind::ArrayAccess(Box::new(ArrayAccess { subject, index })).at(source))
    }
}
