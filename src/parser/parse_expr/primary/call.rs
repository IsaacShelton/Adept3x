use super::Parser;
use crate::{
    ast::{Call, Expr, ExprKind, TypeArg, Using},
    inflow::Inflow,
    name::Name,
    parser::error::ParseError,
    source_files::Source,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_call(
        &mut self,
        name: Name,
        generics: Vec<TypeArg>,
        source: Source,
    ) -> Result<Expr, ParseError> {
        self.parse_call_raw_with(name, generics, vec![])
            .map(|call| ExprKind::Call(Box::new(call)).at(source))
    }

    pub fn parse_call_with(
        &mut self,
        name: Name,
        generics: Vec<TypeArg>,
        prefix_args: Vec<Expr>,
        source: Source,
    ) -> Result<Expr, ParseError> {
        self.parse_call_raw_with(name, generics, prefix_args)
            .map(|call| ExprKind::Call(Box::new(call)).at(source))
    }

    pub fn parse_call_raw(
        &mut self,
        name: Name,
        generics: Vec<TypeArg>,
    ) -> Result<Call, ParseError> {
        self.parse_call_raw_with(name, generics, vec![])
    }

    pub fn parse_call_raw_with(
        &mut self,
        name: Name,
        generics: Vec<TypeArg>,
        prefix_args: Vec<Expr>,
    ) -> Result<Call, ParseError> {
        // function_name(arg1, arg2, arg3)
        //       ^

        let starting_args_len = prefix_args.len();
        let mut args = prefix_args;

        self.parse_token(TokenKind::OpenParen, Some("to begin call argument list"))?;
        self.ignore_newlines();

        while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
            if args.len() > starting_args_len {
                self.parse_token(TokenKind::Comma, Some("to separate arguments"))?;
                self.ignore_newlines();
            }

            args.push(self.parse_expr()?);
            self.ignore_newlines();
        }

        self.parse_token(TokenKind::CloseParen, Some("to end call argument list"))?;

        let mut using = vec![];

        if self.input.peek_is(TokenKind::StaticMember)
            && self.input.peek_nth(1).kind.is_open_paren()
        {
            assert!(self.input.advance().is_static_member());
            assert!(self.input.advance().is_open_paren());

            while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
                if using.len() > starting_args_len {
                    self.parse_token(
                        TokenKind::Comma,
                        Some("to separate implementation arguments"),
                    )?;
                    self.ignore_newlines();
                }

                let name = if self.input.peek().is_identifier()
                    && self.input.peek_nth(1).could_start_type()
                {
                    Some(self.parse_identifier(Some("for implementation parameter name"))?)
                } else {
                    None
                };

                using.push(Using {
                    name,
                    ty: self.parse_type(Some("implementation"), Some("for implementation"))?,
                });
                self.ignore_newlines();
            }

            self.parse_token(TokenKind::CloseParen, Some("to end implementation list"))?;
        }

        Ok(Call {
            name,
            args,
            expected_to_return: None,
            generics,
            using,
        })
    }
}
