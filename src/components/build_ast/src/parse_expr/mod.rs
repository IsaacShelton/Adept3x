mod post;
mod primary;

use super::{ParseError, Parser, is_right_associative, is_terminating_token};
use ast::Expr;
use infinite_iterator::InfinitePeekable;
use token::Token;

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let primary = self.parse_expr_primary()?;
        self.parse_operator_expr(0, primary)
    }
}
