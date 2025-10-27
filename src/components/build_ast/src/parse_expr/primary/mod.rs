mod call;
mod declare_assign;
mod operator;
mod static_member;
mod struct_literal;

use super::{super::error::ParseError, Parser, is_right_associative, is_terminating_token};
use crate::{
    annotation::AnnotationKind, array_last, error::ParseErrorKind, parse_util::into_plain_name,
};
use ast::{
    Block, Conditional, Expr, ExprKind, Integer, SizeOfMode, TypeArg, TypeKind, UnaryMathOperator,
    UnaryOperation, UnaryOperator, While,
};
use infinite_iterator::InfinitePeekable;
use std::ffi::CString;
use std_ext::SmallVec4;
use token::{StringLiteral, StringModifier, Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_expr_primary(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_expr_primary_base()?;
        self.parse_expr_primary_post(expr)
    }

    pub fn parse_expr_primary_base(&mut self) -> Result<Expr, ParseError> {
        let Token { kind, source } = self.input.peek();
        let source = *source;

        match kind {
            TokenKind::Hash => {
                let mut annotations = SmallVec4::new();

                while self.input.peek().is_hash() {
                    annotations.extend(self.parse_annotation_list()?);
                    self.input.ignore_newlines();
                }

                let mut wrapped = self.parse_expr()?;

                for annotation in annotations.iter().rev() {
                    match &annotation.kind {
                        AnnotationKind::Comptime => {
                            wrapped = ExprKind::Comptime(Box::new(wrapped)).at(annotation.source)
                        }
                        _ => return Err(self.unexpected_annotation(annotation, "for expression")),
                    }
                }

                Ok(wrapped)
            }
            TokenKind::TrueKeyword => {
                self.input.advance().kind.unwrap_true_keyword();
                Ok(ExprKind::Boolean(true).at(source))
            }
            TokenKind::FalseKeyword => {
                self.input.advance().kind.unwrap_false_keyword();
                Ok(Expr::new(ExprKind::Boolean(false), source))
            }
            TokenKind::NullKeyword => {
                self.input.advance().kind.unwrap_null_keyword();
                Ok(Expr::new(ExprKind::Null, source))
            }
            TokenKind::Integer(..) => Ok(Expr::new(
                ExprKind::Integer(Integer::Generic(self.input.advance().kind.unwrap_integer())),
                source,
            )),
            TokenKind::Float(..) => Ok(Expr::new(
                ExprKind::Float(self.input.advance().kind.unwrap_float()),
                source,
            )),
            TokenKind::String(StringLiteral {
                modifier: StringModifier::NullTerminated,
                ..
            }) => {
                let Ok(content) = CString::new(self.input.advance().kind.unwrap_string().value)
                else {
                    return Err(ParseErrorKind::CannotContainNulInNullTerminatedString.at(source));
                };

                Ok(Expr::new(ExprKind::NullTerminatedString(content), source))
            }
            TokenKind::String(StringLiteral {
                modifier: StringModifier::Normal,
                ..
            }) => Ok(Expr::new(
                if self.treat_string_literals_as_cstring_literals {
                    ExprKind::NullTerminatedString(
                        CString::new(self.input.advance().kind.unwrap_string().value)
                            .expect("valid null-terminated string"),
                    )
                } else {
                    ExprKind::String(self.input.advance().kind.unwrap_string().value)
                },
                source,
            )),
            TokenKind::String(StringLiteral {
                modifier: StringModifier::CharLiteral,
                ..
            }) => {
                let content = self.input.advance().kind.unwrap_string().value;

                if content.len() != 1 {
                    return Err(ParseErrorKind::CharLiteralCannotBeLargerThanOneByte.at(source));
                }

                Ok(Expr::new(
                    ExprKind::CharLiteral(content.as_bytes()[0]),
                    source,
                ))
            }
            TokenKind::String(StringLiteral {
                modifier: StringModifier::RuneLiteral,
                ..
            }) => {
                let content = self.input.advance().kind.unwrap_string().value;
                Ok(Expr::new(ExprKind::Char(content), source))
            }
            TokenKind::OpenParen => {
                self.input.advance().kind.unwrap_open_paren();
                let inner = self.parse_expr()?;
                self.input
                    .expect(TokenKind::CloseParen, "to close nested expression")?;
                Ok(inner)
            }
            TokenKind::StructKeyword | TokenKind::UnionKeyword | TokenKind::EnumKeyword => {
                self.parse_struct_literal()
            }
            TokenKind::Polymorph(_) => {
                let polymorph = self.input.eat_polymorph().unwrap();
                let subject = TypeKind::Polymorph(polymorph).at(source);
                self.parse_static_member_with_type(subject, source)
            }
            TokenKind::Identifier(_) | TokenKind::OldNamespacedIdentifier(_) => {
                // TODO: CLEANUP: This should be cleaned up once we have proper
                // namespaces and generic parsing that applies to all cases

                let name_path = self.parse_name_path(None::<&str>).unwrap();
                let generics = self.parse_type_args()?;

                match self.input.peek().kind {
                    TokenKind::StaticMember => {
                        self.parse_static_member(name_path, generics, source)
                    }
                    TokenKind::OpenCurly => {
                        let peek = &self.input.peek_nth(1).kind;

                        if peek.is_extend() || peek.is_colon() {
                            let ast_type =
                                self.parse_type_from_parts(name_path, generics, source)?;
                            self.parse_struct_literal_with(ast_type)
                        } else {
                            let last_two =
                                array_last::<2, 4, _>(self.input.peek_n()).map(|token| &token.kind);

                            match &last_two[..] {
                                [TokenKind::Colon, ..]
                                | [TokenKind::Identifier(_), TokenKind::Colon, ..] => {
                                    let ast_type =
                                        self.parse_type_from_parts(name_path, generics, source)?;
                                    self.parse_struct_literal_with(ast_type)
                                }
                                _ => Ok(Expr::new(ExprKind::Variable(name_path), source)),
                            }
                        }
                    }
                    TokenKind::OpenParen => self.parse_call(name_path, generics, source),
                    TokenKind::DeclareAssign => {
                        if !generics.is_empty() {
                            return Err(ParseErrorKind::GenericsNotSupportedHere.at(source));
                        }

                        self.parse_declare_assign(into_plain_name(name_path, source)?, source)
                    }
                    _ => {
                        if !generics.is_empty() {
                            let mut generics = generics;
                            let mut generics = generics.drain(..);

                            if let Some("sizeof") = name_path.as_plain_str() {
                                let Some(first_arg) = generics.next() else {
                                    return Err(ParseErrorKind::Other {
                                        message: "Expected type argument to sizeof macro".into(),
                                    }
                                    .at(source));
                                };

                                // Decide meaning of arguments: `sizeof<"target", T>` vs `sizeof<T>`
                                let (arg, mode) = if let Some(second_arg) = generics.next() {
                                    (second_arg, Some(first_arg))
                                } else {
                                    (first_arg, None)
                                };

                                let mode = match mode {
                                    Some(TypeArg::Expr(Expr {
                                        kind: ExprKind::String(mode),
                                        ..
                                    })) => match mode.as_str() {
                                        "target" => Some(SizeOfMode::Target),
                                        "compilation" => Some(SizeOfMode::Compilation),
                                        _ => {
                                            return Err(ParseError::other(
                                                format!(
                                                    "Mode must be either 'target' or 'compilation'"
                                                ),
                                                source,
                                            ));
                                        }
                                    },
                                    Some(..) => {
                                        return Err(ParseError::other(
                                            "Mode for sizeof must be a string",
                                            source,
                                        ));
                                    }
                                    None => None,
                                };

                                let expr = if let TypeArg::Type(ty) = arg {
                                    ExprKind::SizeOf(Box::new(ty), mode).at(source)
                                } else if let TypeArg::Expr(value) = arg {
                                    ExprKind::SizeOfValue(Box::new(value), mode).at(source)
                                } else {
                                    return Err(ParseError::other(
                                        "Cannot get size of compile-time value that isn't type or expression",
                                        source,
                                    ));
                                };

                                if generics.next().is_some() {
                                    return Err(ParseError::other(
                                        "Too many arguments to sizeof macro",
                                        source,
                                    ));
                                };

                                return Ok(expr);
                            }

                            return Err(ParseError::other(
                                format!("Macro '{}' does not exist", name_path.to_string()),
                                source,
                            ));
                        }

                        Ok(Expr::new(ExprKind::Variable(name_path), source))
                    }
                }
            }
            TokenKind::Not => {
                self.input.advance();

                Ok(ExprKind::UnaryOperation(Box::new(UnaryOperation::new_math(
                    UnaryMathOperator::Not,
                    self.parse_expr_primary()?,
                )))
                .at(source))
            }
            TokenKind::BitComplement => {
                self.input.advance();

                Ok(ExprKind::UnaryOperation(Box::new(UnaryOperation::new_math(
                    UnaryMathOperator::BitComplement,
                    self.parse_expr_primary()?,
                )))
                .at(source))
            }
            TokenKind::Subtract => {
                self.input.advance();

                let mut inside = self.parse_expr_primary()?;

                match &mut inside.kind {
                    ExprKind::Integer(Integer::Generic(value)) => {
                        *value = -(&*value);
                        Ok(inside)
                    }
                    ExprKind::Float(value) => {
                        *value = -*value;
                        Ok(inside)
                    }
                    _ => Ok(ExprKind::UnaryOperation(Box::new(UnaryOperation::new_math(
                        UnaryMathOperator::Negate,
                        inside,
                    )))
                    .at(source)),
                }
            }
            TokenKind::AddressOf => {
                self.input.advance();

                Ok(ExprKind::UnaryOperation(Box::new(UnaryOperation::new(
                    UnaryOperator::AddressOf,
                    self.parse_expr_primary()?,
                )))
                .at(source))
            }
            TokenKind::Dereference => {
                self.input.advance();

                Ok(ExprKind::UnaryOperation(Box::new(UnaryOperation::new(
                    UnaryOperator::Dereference,
                    self.parse_expr_primary()?,
                )))
                .at(source))
            }
            TokenKind::IfKeyword => {
                self.input.advance().kind.unwrap_if_keyword();
                self.ignore_newlines();

                let condition = self.parse_expr()?;
                let stmts = self.parse_block("'if'")?;
                let mut conditions = vec![(condition, Block::new(stmts))];

                while self.input.peek_is(TokenKind::ElifKeyword) {
                    self.input.advance().kind.unwrap_elif_keyword();
                    self.ignore_newlines();

                    let condition = self.parse_expr()?;
                    conditions.push((condition, Block::new(self.parse_block("'elif'")?)));
                }

                let otherwise = self
                    .input
                    .peek_is(TokenKind::ElseKeyword)
                    .then(|| {
                        self.input.advance().kind.unwrap_else_keyword();
                        Ok(Block::new(self.parse_block("'else'")?))
                    })
                    .transpose()?;

                let conditions = conditions.into_boxed_slice();

                let conditional = Conditional {
                    conditions,
                    otherwise,
                    conform_behavior: self.conform_behavior,
                };

                Ok(Expr::new(ExprKind::Conditional(conditional), source))
            }
            TokenKind::WhileKeyword => {
                self.input.advance().kind.unwrap_while_keyword();
                self.ignore_newlines();

                let condition = self.parse_expr()?;
                let stmts = self.parse_block("'while'")?;

                Ok(Expr::new(
                    ExprKind::While(Box::new(While {
                        condition,
                        block: Block::new(stmts),
                    })),
                    source,
                ))
            }
            TokenKind::BreakKeyword => {
                self.input.advance().kind.unwrap_break_keyword();
                Ok(ExprKind::Break.at(source))
            }
            TokenKind::ContinueKeyword => {
                self.input.advance().kind.unwrap_continue_keyword();
                Ok(ExprKind::Continue.at(source))
            }
            TokenKind::Label(_) => {
                let name = self.input.advance().kind.unwrap_label();
                Ok(ExprKind::LabelLiteral(name).at(source))
            }
            unexpected => Err(ParseError {
                kind: match unexpected {
                    TokenKind::Error(message) => ParseErrorKind::Lexical {
                        message: message.into(),
                    },
                    _ => ParseErrorKind::UnexpectedToken {
                        unexpected: unexpected.to_string(),
                    },
                },
                source,
            }),
        }
    }
}
