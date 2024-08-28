use super::{
    error::{ParseError, ParseErrorKind},
    Parser,
};
use crate::{
    ast::{FixedArray, Type, TypeKind},
    inflow::Inflow,
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
            "ptr" => Ok(TypeKind::Pointer(Box::new(
                if self.input.eat(TokenKind::OpenAngle) {
                    let inner = self.parse_type(None::<&str>, None::<&str>)?;
                    self.parse_type_parameters_close()?;
                    inner
                } else {
                    TypeKind::Void.at(source)
                },
            ))),
            "array" => {
                if !self.input.eat(TokenKind::OpenAngle) {
                    return Err(ParseError {
                        kind: ParseErrorKind::ExpectedTypeParameters,
                        source,
                    });
                }

                let count = self.parse_expr()?;

                if !self.input.eat(TokenKind::Comma) {
                    return Err(ParseError {
                        kind: ParseErrorKind::ExpectedCommaInTypeParameters,
                        source: self.source_here(),
                    });
                }

                let inner = self.parse_type(None::<&str>, None::<&str>)?;
                self.parse_type_parameters_close()?;

                Ok(TypeKind::FixedArray(Box::new(FixedArray {
                    ast_type: inner,
                    count,
                })))
            }
            identifier => Ok(TypeKind::Named(identifier.into())),
        }?;

        Ok(Type::new(type_kind, source))
    }

    /// Parses closing '>' brackets of type parameters.
    /// This function may partially consume tokens, so be
    /// aware that any previously peeked tokens may no longer be in
    /// the same lookahead position after calling this function.
    fn parse_type_parameters_close(&mut self) -> Result<(), ParseError> {
        let closer = self.input.advance();

        /// Sub-function for properly handling trailing `=` signs
        /// resulting from partially consuming '>'-like tokens.
        fn merge_trailing_equals<I: Inflow<Token>>(
            parser: &mut Parser<I>,
            closer: &Token,
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
            TokenKind::GreaterThan => Ok(()),
            TokenKind::RightShift => {
                self.input
                    .unadvance(TokenKind::GreaterThan.at(closer.source.shift_column(1)));
                Ok(())
            }
            TokenKind::LogicalRightShift => {
                self.input
                    .unadvance(TokenKind::RightShift.at(closer.source.shift_column(1)));
                Ok(())
            }
            TokenKind::RightShiftAssign => {
                merge_trailing_equals(self, &closer, 2);

                self.input
                    .unadvance(TokenKind::GreaterThan.at(closer.source.shift_column(1)));
                Ok(())
            }
            TokenKind::LogicalRightShiftAssign => {
                merge_trailing_equals(self, &closer, 3);

                self.input
                    .unadvance(TokenKind::RightShift.at(closer.source.shift_column(1)));
                Ok(())
            }
            TokenKind::GreaterThanEq => {
                merge_trailing_equals(self, &closer, 1);
                Ok(())
            }
            _ => Err(self.unexpected_token(&closer)),
        }
    }
}
