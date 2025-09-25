use super::Parser;
use crate::error::ParseError;
use ast::{DeclareAssign, Expr, ExprKind};
use infinite_iterator::InfinitePeekable;
use source_files::Source;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_declare_assign(
        &mut self,
        name: Box<str>,
        source: Source,
    ) -> Result<Expr, ParseError> {
        // variable_name := value
        //               ^

        self.input.expect(
            TokenKind::DeclareAssign,
            Some("for variable declaration assignment"),
        )?;
        self.ignore_newlines();

        Ok(ExprKind::DeclareAssign(Box::new(DeclareAssign {
            name,
            value: self.parse_expr()?,
        }))
        .at(source))
    }
}
