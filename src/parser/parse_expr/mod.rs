mod post;
mod primary;

use super::{error::ParseError, is_right_associative, is_terminating_token, Parser};
use crate::{ast::Expr, inflow::Inflow, token::Token};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let primary = self.parse_expr_primary()?;
        self.parse_operator_expr(0, primary)
    }
}
