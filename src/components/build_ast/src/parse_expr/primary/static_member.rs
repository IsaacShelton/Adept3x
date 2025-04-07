use super::Parser;
use crate::error::{ParseError, ParseErrorKind};
use ast::{Expr, ExprKind, Name, StaticMemberCall, StaticMemberValue, Type, TypeArg};
use inflow::Inflow;
use source_files::Source;
use token::{Token, TokenKind};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_static_member(
        &mut self,
        type_name: Name,
        generics: Vec<TypeArg>,
        source: Source,
    ) -> Result<Expr, ParseError> {
        // EnumName::EnumVariant
        //         ^

        let subject = self.parse_type_from_parts(type_name, generics, source)?;
        self.parse_static_member_with_type(subject, source)
    }

    pub fn parse_static_member_with_type(
        &mut self,
        subject: Type,
        source: Source,
    ) -> Result<Expr, ParseError> {
        // EnumName::EnumVariant
        //         ^

        self.parse_token(TokenKind::StaticMember, Some("for static member access"))?;

        let action_source = self.source_here();
        let action_name = self
            .input
            .eat_identifier()
            .ok_or_else(|| ParseErrorKind::ExpectedEnumMemberName.at(action_source))?;

        Ok(
            if self.input.peek_is(TokenKind::OpenParen) || self.input.peek_is(TokenKind::OpenAngle)
            {
                let name = Name::plain(action_name);
                let generics = self.parse_type_args()?;

                ExprKind::StaticMemberCall(Box::new(StaticMemberCall {
                    subject,
                    call: self.parse_call_raw(name, generics)?,
                    call_source: action_source,
                    source,
                }))
            } else {
                ExprKind::StaticMemberValue(Box::new(StaticMemberValue {
                    subject,
                    value: action_name,
                    value_source: action_source,
                    source,
                }))
            }
            .at(source),
        )
    }
}
