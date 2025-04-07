use super::Parser;
use crate::error::ParseError;
use ast::{DeclareAssign, Expr, ExprKind};
use infinite_iterator::InfinitePeekable;
use source_files::Source;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_declare_assign(
        &mut self,
        variable_name: String,
        source: Source,
    ) -> Result<Expr, ParseError> {
        // variable_name := value
        //               ^

        self.parse_token(
            TokenKind::DeclareAssign,
            Some("for variable declaration assignment"),
        )?;
        self.ignore_newlines();

        let value = self.parse_expr()?;

        Ok(ExprKind::DeclareAssign(Box::new(DeclareAssign {
            name: variable_name,
            value,
        }))
        .at(source))
    }
}
