use super::Parser;
use crate::{
    ast::{Expr, ExprKind, Privacy},
    inflow::Inflow,
    parser::{error::ParseError, parse_util::into_plain_name},
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_member(&mut self, subject: Expr) -> Result<Expr, ParseError> {
        // subject.field_name
        //        ^

        let source = self.parse_token(TokenKind::Member, Some("for member expression"))?;
        let member_name = self.parse_name(Some("for member name"))?;

        if self.input.peek_is(TokenKind::OpenParen) || self.input.peek_is(TokenKind::OpenAngle) {
            let generics = self.parse_generics()?;
            self.parse_call_with(member_name, generics, vec![subject], source)
        } else {
            Ok(ExprKind::Member(
                Box::new(subject),
                into_plain_name(member_name, source)?,
                Privacy::Public,
            )
            .at(source))
        }
    }
}
