use super::Parser;
use crate::{
    ast::{CompileTimeArgument, EnumMemberLiteral, Expr, ExprKind},
    inflow::Inflow,
    name::Name,
    parser::error::{ParseError, ParseErrorKind},
    source_files::Source,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_static_member(
        &mut self,
        enum_name: Name,
        generics: Vec<CompileTimeArgument>,
        source: Source,
    ) -> Result<Expr, ParseError> {
        // EnumName::EnumVariant
        //    ^

        if !generics.is_empty() {
            return Err(ParseErrorKind::GenericsNotSupportedHere.at(source));
        }

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
