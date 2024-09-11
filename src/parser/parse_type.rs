use super::{
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    ast::{CompileTimeArgument, Type, TypeKind},
    inflow::Inflow,
    source_files::Source,
    token::{Token, TokenKind},
};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_type(
        &mut self,
        prefix: Option<impl ToString>,
        for_reason: Option<impl ToString>,
    ) -> Result<Type, ParseError> {
        let source = self.input.peek().source;
        let token = self.input.advance();

        let TokenKind::Identifier(identifier) = token.kind else {
            return Err(ParseError {
                kind: ParseErrorKind::ExpectedType {
                    prefix: prefix.map(|prefix| prefix.to_string()),
                    for_reason: for_reason.map(|for_reason| for_reason.to_string()),
                    got: token.to_string(),
                },
                source,
            });
        };

        let generics = self.parse_generics()?;
        self.parse_type_from_parts(identifier, generics, source)
    }

    pub fn parse_generics(&mut self) -> Result<Vec<CompileTimeArgument>, ParseError> {
        let mut generics = vec![];

        if !self.input.eat(TokenKind::OpenAngle) {
            return Ok(generics);
        }

        loop {
            if self.parse_type_parameters_close().is_some() {
                break;
            } else if self.input.peek_is(TokenKind::EndOfFile) {
                // TODO: Improve error message
                return Err(self.unexpected_token_is_next());
            }

            if !generics.is_empty() && !self.input.eat(TokenKind::Comma) {
                // TODO: Improve error message
                return Err(self.unexpected_token_is_next());
            }

            generics.push(if self.input.peek().could_start_type() {
                CompileTimeArgument::Type(
                    self.parse_type(None::<&str>, Some("for compile time argument"))?,
                )
            } else {
                CompileTimeArgument::Expr(self.parse_expr()?)
            });
        }

        Ok(generics)
    }

    pub fn parse_type_from_parts(
        &mut self,
        identifier: String,
        generics: Vec<CompileTimeArgument>,
        source: Source,
    ) -> Result<Type, ParseError> {
        let type_kind = match identifier.as_str() {
            "bool" => Ok(TypeKind::Boolean),
            "char" => Ok(TypeKind::char()),
            "schar" => Ok(TypeKind::schar()),
            "uchar" => Ok(TypeKind::uchar()),
            "short" => Ok(TypeKind::short()),
            "ushort" => Ok(TypeKind::ushort()),
            "int" => Ok(TypeKind::int()),
            "uint" => Ok(TypeKind::uint()),
            "long" => Ok(TypeKind::long()),
            "ulong" => Ok(TypeKind::ulong()),
            "longlong" => Ok(TypeKind::longlong()),
            "ulonglong" => Ok(TypeKind::ulonglong()),
            "i8" => Ok(TypeKind::i8()),
            "u8" => Ok(TypeKind::u8()),
            "i16" => Ok(TypeKind::i16()),
            "u16" => Ok(TypeKind::u16()),
            "i32" => Ok(TypeKind::i32()),
            "u32" => Ok(TypeKind::u32()),
            "i64" => Ok(TypeKind::i64()),
            "u64" => Ok(TypeKind::u64()),
            "f32" | "float" => Ok(TypeKind::f32()),
            "f64" | "double" => Ok(TypeKind::f64()),
            "void" => Ok(TypeKind::Void),
            "ptr" => {
                if generics.len() == 1 {
                    if let CompileTimeArgument::Type(inner) = generics.into_iter().next().unwrap() {
                        Ok(TypeKind::Pointer(Box::new(inner)))
                    } else {
                        Err(ParseError {
                            kind: ParseErrorKind::ExpectedTypeParameterToBeAType {
                                name: identifier,
                                word_for_nth: "first".into(),
                            },
                            source,
                        })
                    }
                } else {
                    Err(ParseError {
                        kind: ParseErrorKind::IncorrectNumberOfTypeParametersFor {
                            name: identifier,
                            expected: 1,
                            got: generics.len(),
                        },
                        source,
                    })
                }
            }
            "array" => {
                // TODO: Update fixed array type to use compile time arguments
                todo!("array<$N, $T> not updated yet to use compile time arguments");

                // Ok(TypeKind::FixedArray(Box::new(FixedArray {
                //     ast_type: inner,
                //     count,
                // })))
            }
            identifier => Ok(TypeKind::Named(identifier.into())),
        }?;

        Ok(Type::new(type_kind, source))
    }

    /// Parses closing '>' brackets of type parameters.
    /// This function may partially consume tokens, so be
    /// aware that any previously peeked tokens may no longer be in
    /// the same lookahead position after calling this function.
    fn parse_type_parameters_close(&mut self) -> Option<()> {
        let closer = self.input.peek();
        let source = closer.source;

        /// Sub-function for properly handling trailing `=` signs
        /// resulting from partially consuming '>'-like tokens.
        fn merge_trailing_equals<I: Inflow<Token>>(
            parser: &mut Parser<I>,
            closer: Token,
            column_offset: u32,
        ) {
            if parser.input.eat(TokenKind::Assign) {
                parser
                    .input
                    .unadvance(TokenKind::Equals.at(closer.source.shift_column(column_offset)));
            } else {
                parser
                    .input
                    .unadvance(TokenKind::Assign.at(closer.source.shift_column(column_offset)));
            }
        }

        match &closer.kind {
            TokenKind::GreaterThan => {
                self.input.advance();
                Some(())
            }
            TokenKind::RightShift => {
                self.input.advance();
                self.input
                    .unadvance(TokenKind::GreaterThan.at(source.shift_column(1)));
                Some(())
            }
            TokenKind::LogicalRightShift => {
                self.input.advance();
                self.input
                    .unadvance(TokenKind::RightShift.at(source.shift_column(1)));
                Some(())
            }
            TokenKind::RightShiftAssign => {
                let closer = self.input.advance();
                merge_trailing_equals(self, closer, 2);
                self.input
                    .unadvance(TokenKind::GreaterThan.at(source.shift_column(1)));
                Some(())
            }
            TokenKind::LogicalRightShiftAssign => {
                let closer = self.input.advance();
                merge_trailing_equals(self, closer, 3);
                self.input
                    .unadvance(TokenKind::RightShift.at(source.shift_column(1)));
                Some(())
            }
            TokenKind::GreaterThanEq => {
                let closer = self.input.advance();
                merge_trailing_equals(self, closer, 1);
                Some(())
            }
            _ => None,
        }
    }
}
