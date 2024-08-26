use super::Parser;
use crate::{
    ast::{Expr, ExprKind},
    inflow::Inflow,
    parser::error::ParseError,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_member(&mut self, subject: Expr) -> Result<Expr, ParseError> {
        // subject.field_name
        //        ^

        let source = self.parse_token(TokenKind::Member, Some("for member expression"))?;
        let field_name = self.parse_identifier(Some("for field name"))?;

        Ok(ExprKind::Member(Box::new(subject), field_name).at(source))
    }
}
