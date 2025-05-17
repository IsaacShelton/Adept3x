use super::Parser;
use crate::error::ParseError;
use ast::{Call, Expr, ExprKind, Name, TypeArg, Using};
use infinite_iterator::InfinitePeekable;
use source_files::Source;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
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

        self.input
            .expect(TokenKind::OpenParen, "to begin call argument list")?;
        self.ignore_newlines();

        while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
            if args.len() > starting_args_len {
                self.input
                    .expect(TokenKind::Comma, "to separate arguments")?;
                self.ignore_newlines();
            }

            args.push(self.parse_expr()?);
            self.ignore_newlines();
        }

        self.input
            .expect(TokenKind::CloseParen, "to end call argument list")?;

        let mut using = vec![];

        if self.input.peek_is(TokenKind::StaticMember)
            && self.input.peek_nth(1).kind.is_open_paren()
        {
            assert!(self.input.advance().is_static_member());
            assert!(self.input.advance().is_open_paren());

            self.ignore_newlines();

            while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
                if using.len() > starting_args_len {
                    self.input
                        .expect(TokenKind::Comma, "to separate implementation arguments")?;
                    self.ignore_newlines();
                }

                let name = (self.input.peek().is_identifier()
                    && self.input.peek_nth(1).could_start_type())
                .then(|| self.parse_identifier_keep_location("for implementation parameter name"))
                .transpose()?;

                using.push(Using {
                    name,
                    ty: self.parse_type("implementation", "for implementation")?,
                });
                self.ignore_newlines();
            }

            self.input
                .expect(TokenKind::CloseParen, "to end implementation list")?;
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
