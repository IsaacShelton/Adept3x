use super::Parser;
use crate::{
    ast::{DeclareAssign, Expr, ExprKind},
    inflow::Inflow,
    parser::error::ParseError,
    source_files::Source,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
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
