use super::{error::ParseError, Parser};
use crate::{
    ast::{Param, Params},
    inflow::Inflow,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_func_params(&mut self) -> Result<Params, ParseError> {
        // (arg1 Type1, arg2 Type2, arg3 Type3)
        // ^

        let mut required = vec![];
        let mut is_cstyle_vararg = false;

        self.parse_token(TokenKind::OpenParen, Some("to begin function parameters"))?;
        self.ignore_newlines();

        while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
            // Parse argument

            if !required.is_empty() {
                self.parse_token(TokenKind::Comma, Some("after parameter"))?;
                self.ignore_newlines();
            }

            if self.input.peek_is(TokenKind::Ellipsis) {
                is_cstyle_vararg = true;
                self.input.advance();
                self.ignore_newlines();
                break;
            }

            let name = self.parse_identifier(Some("for parameter name"))?;
            self.ignore_newlines();
            let ast_type = self.parse_type(None::<&str>, Some("for parameter"))?;
            self.ignore_newlines();
            required.push(Param::named(name, ast_type));
        }

        self.parse_token(TokenKind::CloseParen, Some("to end function parameters"))?;

        Ok(Params {
            required,
            is_cstyle_vararg,
        })
    }
}
