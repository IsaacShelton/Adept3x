use super::Parser;
use crate::{
    ast::{DeclareAssign, Expr, ExprKind},
    inflow::Inflow,
    parser::error::ParseError,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_declare_assign(&mut self) -> Result<Expr, ParseError> {
        // variable_name := value
        //               ^

        let (variable_name, source) =
            self.parse_identifier_keep_location(Some("for function call"))?;

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
