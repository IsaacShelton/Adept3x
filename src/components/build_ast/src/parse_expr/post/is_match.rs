use super::Parser;
use crate::error::ParseError;
use ast::{Expr, ExprKind};
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_is_match(&mut self, subject: Expr) -> Result<Expr, ParseError> {
        // subject is Variant
        //         ^

        let source = self.input.here();

        self.input
            .expect(TokenKind::IsKeyword, "for 'is' expression")?;

        let variant_name = self.parse_identifier("for variant name")?;

        Ok(ExprKind::Is(Box::new(subject), variant_name).at(source))
    }
}
