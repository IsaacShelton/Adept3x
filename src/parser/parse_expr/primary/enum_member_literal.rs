use super::Parser;
use crate::{
    ast::{EnumMemberLiteral, Expr, ExprKind},
    inflow::Inflow,
    parser::error::{ParseError, ParseErrorKind},
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_enum_member_literal(&mut self) -> Result<Expr, ParseError> {
        // EnumName::EnumVariant
        //    ^

        let source = self.source_here();
        let enum_name = self
            .input
            .eat_identifier()
            .ok_or_else(|| ParseErrorKind::ExpectedEnumName.at(source))?;

        self.parse_token(TokenKind::Namespace, Some("for enum member literal"))?;

        let variant_source = self.source_here();
        let variant_name = self
            .input
            .eat_identifier()
            .ok_or_else(|| ParseErrorKind::ExpectedEnumName.at(variant_source))?;

        Ok(ExprKind::EnumMemberLiteral(Box::new(EnumMemberLiteral {
            enum_name,
            variant_name,
            source,
        }))
        .at(source))
    }
}
