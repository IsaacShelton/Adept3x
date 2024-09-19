use super::Parser;
use crate::{
    ast::{EnumMemberLiteral, Expr, ExprKind},
    inflow::Inflow,
    parser::error::{ParseError, ParseErrorKind},
    source_files::Source,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_enum_member_literal(
        &mut self,
        enum_name: String,
        source: Source,
    ) -> Result<Expr, ParseError> {
        // EnumName::EnumVariant
        //    ^

        self.parse_token(TokenKind::StaticMember, Some("for enum member literal"))?;

        let variant_source = self.source_here();
        let variant_name = self
            .input
            .eat_identifier()
            .ok_or_else(|| ParseErrorKind::ExpectedEnumMemberName.at(variant_source))?;

        Ok(ExprKind::EnumMemberLiteral(Box::new(EnumMemberLiteral {
            enum_name,
            variant_name,
            source,
        }))
        .at(source))
    }
}
