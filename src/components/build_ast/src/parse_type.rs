use super::{
    Parser,
    error::{ParseError, ParseErrorKind},
};
use ast::{Name, Type, TypeArg, TypeKind};
use infinite_iterator::InfinitePeekable;
use source_files::Source;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_type(
        &mut self,
        prefix: Option<impl ToString>,
        for_reason: Option<impl ToString>,
    ) -> Result<Type, ParseError> {
        let source = self.input.peek().source;
        let token = self.input.peek().clone();

        let Ok(name) = self.parse_name(None::<&str>) else {
            if token.kind.is_polymorph() {
                return Ok(TypeKind::Polymorph(token.kind.unwrap_polymorph()).at(source));
            }

            return Err(ParseErrorKind::ExpectedType {
                prefix: prefix.map(|prefix| prefix.to_string()),
                for_reason: for_reason.map(|for_reason| for_reason.to_string()),
                got: token.to_string(),
            }
            .at(source));
        };

        let generics = self.parse_type_args()?;
        self.parse_type_from_parts(name, generics, source)
    }

    pub fn parse_type_args(&mut self) -> Result<Vec<TypeArg>, ParseError> {
        let mut generics = vec![];

        if self.input.eat(TokenKind::ShortGeneric) {
            return Ok(vec![self.parse_type_arg()?]);
        }

        if !self.input.eat(TokenKind::OpenAngle) {
            return Ok(generics);
        }

        loop {
            if self.parse_type_param_close().is_some() {
                break;
            } else if self.input.peek_is(TokenKind::EndOfFile) {
                // TODO: Improve error message
                return Err(self.unexpected_token_is_next());
            }

            if !generics.is_empty() && !self.input.eat(TokenKind::Comma) {
                // TODO: Improve error message
                return Err(self.unexpected_token_is_next());
            }

            generics.push(self.parse_type_arg()?);
        }

        Ok(generics)
    }

    pub fn parse_type_arg(&mut self) -> Result<TypeArg, ParseError> {
        Ok(if self.input.peek().could_start_type() {
            TypeArg::Type(self.parse_type(None::<&str>, Some("for compile time argument"))?)
        } else {
            TypeArg::Expr(self.parse_expr_primary_base()?)
        })
    }

    pub fn parse_type_from_parts(
        &mut self,
        name: Name,
        generics: Vec<TypeArg>,
        source: Source,
    ) -> Result<Type, ParseError> {
        let type_kind = match name.as_plain_str() {
            Some("bool") => Ok(TypeKind::Boolean),
            Some("char") => Ok(TypeKind::char()),
            Some("schar") => Ok(TypeKind::schar()),
            Some("uchar") => Ok(TypeKind::uchar()),
            Some("short") => Ok(TypeKind::short()),
            Some("ushort") => Ok(TypeKind::ushort()),
            Some("int") => Ok(TypeKind::int()),
            Some("uint") => Ok(TypeKind::uint()),
            Some("long") => Ok(TypeKind::long()),
            Some("ulong") => Ok(TypeKind::ulong()),
            Some("longlong") => Ok(TypeKind::longlong()),
            Some("ulonglong") => Ok(TypeKind::ulonglong()),
            Some("i8") => Ok(TypeKind::i8()),
            Some("u8") => Ok(TypeKind::u8()),
            Some("i16") => Ok(TypeKind::i16()),
            Some("u16") => Ok(TypeKind::u16()),
            Some("i32") => Ok(TypeKind::i32()),
            Some("u32") => Ok(TypeKind::u32()),
            Some("i64") => Ok(TypeKind::i64()),
            Some("u64") => Ok(TypeKind::u64()),
            Some("usize") => Ok(TypeKind::usize()),
            Some("isize") => Ok(TypeKind::isize()),
            Some("f32" | "float") => Ok(TypeKind::f32()),
            Some("f64" | "double") => Ok(TypeKind::f64()),
            Some("void") => Ok(TypeKind::Void),
            Some("never") => Ok(TypeKind::Never),
            Some("ptr") => {
                if generics.len() == 1 {
                    if let TypeArg::Type(inner) = generics.into_iter().next().unwrap() {
                        Ok(TypeKind::Ptr(Box::new(inner)))
                    } else {
                        Err(ParseError {
                            kind: ParseErrorKind::ExpectedTypeParameterToBeAType {
                                name: name.to_string(),
                                word_for_nth: "first".into(),
                            },
                            source,
                        })
                    }
                } else {
                    Err(ParseError {
                        kind: ParseErrorKind::IncorrectNumberOfTypeParametersFor {
                            name: name.to_string(),
                            expected: 1,
                            got: generics.len(),
                        },
                        source,
                    })
                }
            }
            Some("array") => {
                // TODO: Update fixed array type to use compile time arguments
                todo!("array<$N, $T> not updated yet to use compile time arguments");

                // Ok(TypeKind::FixedArray(Box::new(FixedArray {
                //     ast_type: inner,
                //     count,
                // })))
            }
            _ => Ok(TypeKind::Named(name, generics)),
        }?;

        Ok(Type::new(type_kind, source))
    }

    /// Parses closing '>' brackets of type parameters.
    /// This function may partially consume tokens, so be
    /// aware that any previously peeked tokens may no longer be in
    /// the same lookahead position after calling this function.
    fn parse_type_param_close(&mut self) -> Option<()> {
        let closer = self.input.peek();
        let source = closer.source;

        /// Sub-function for properly handling trailing `=` signs
        /// resulting from partially consuming '>'-like tokens.
        fn merge_trailing_equals<I: InfinitePeekable<Token>>(
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
