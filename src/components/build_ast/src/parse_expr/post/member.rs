use super::Parser;
use crate::{error::ParseError, parse_util::into_plain_name};
use ast::{Expr, ExprKind};
use attributes::Privacy;
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_member(&mut self, subject: Expr) -> Result<Expr, ParseError> {
        // subject.field_name
        //        ^

        let source = self.input.here();

        self.input
            .expect(TokenKind::Member, "for member expression")?;

        let member_name = self.parse_name("for member name")?;

        let generics = self.parse_type_args()?;

        if !generics.is_empty()
            || self.input.peek_is(TokenKind::OpenParen)
            || self.input.peek_is(TokenKind::OpenAngle)
        {
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
