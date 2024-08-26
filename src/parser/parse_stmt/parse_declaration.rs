use super::{super::error::ParseError, Parser};
use crate::{
    ast::{Declaration, Stmt, StmtKind},
    inflow::Inflow,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_declaration(&mut self) -> Result<Stmt, ParseError> {
        let (name, source) = self.parse_identifier_keep_location(Some("for variable name"))?;

        let variable_type = self.parse_type(None::<&str>, Some("for variable type"))?;

        let initial_value = self
            .input
            .eat(TokenKind::Assign)
            .then(|| self.parse_expr())
            .transpose()?;

        self.ignore_newlines();

        Ok(Stmt::new(
            StmtKind::Declaration(Box::new(Declaration {
                name,
                ast_type: variable_type,
                initial_value,
            })),
            source,
        ))
    }
}
