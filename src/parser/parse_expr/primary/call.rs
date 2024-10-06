use super::Parser;
use crate::{
    ast::{Call, CompileTimeArgument, Expr, ExprKind},
    inflow::Inflow,
    name::Name,
    parser::error::ParseError,
    source_files::Source,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_call(
        &mut self,
        function_name: Name,
        generics: Vec<CompileTimeArgument>,
        source: Source,
    ) -> Result<Expr, ParseError> {
        // function_name(arg1, arg2, arg3)
        //       ^

        self.parse_call_with(function_name, generics, vec![], source)
    }

    pub fn parse_call_with(
        &mut self,
        function_name: Name,
        generics: Vec<CompileTimeArgument>,
        prefix_args: Vec<Expr>,
        source: Source,
    ) -> Result<Expr, ParseError> {
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

        Ok(ExprKind::Call(Box::new(Call {
            function_name,
            arguments: args,
            expected_to_return: None,
            generics,
        }))
        .at(source))
    }
}
