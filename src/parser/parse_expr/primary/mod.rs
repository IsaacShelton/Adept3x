mod call;
mod declare_assign;
mod enum_member_literal;
mod operator;
mod structure_literal;

use super::{super::error::ParseError, is_right_associative, is_terminating_token, Parser};
use crate::{
    ast::{Block, Conditional, Expr, ExprKind, Integer, UnaryOperation, UnaryOperator, While},
    inflow::Inflow,
    parser::{array_last, error::ParseErrorKind},
    token::{StringLiteral, StringModifier, Token, TokenKind},
};
use std::ffi::CString;

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_expr_primary(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_expr_primary_base()?;
        self.parse_expr_primary_post(expr)
    }

    pub fn parse_expr_primary_base(&mut self) -> Result<Expr, ParseError> {
        let Token { kind, source } = self.input.peek();
        let source = *source;

        match kind {
            TokenKind::TrueKeyword => {
                self.input.advance().kind.unwrap_true_keyword();
                Ok(ExprKind::Boolean(true).at(source))
            }
            TokenKind::FalseKeyword => {
                self.input.advance().kind.unwrap_false_keyword();
                Ok(Expr::new(ExprKind::Boolean(false), source))
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
            }) => Ok(Expr::new(
                ExprKind::NullTerminatedString(
                    CString::new(self.input.advance().kind.unwrap_string().value)
                        .expect("valid null-terminated string"),
                ),
                source,
            )),
            TokenKind::String(StringLiteral {
                modifier: StringModifier::Normal,
                ..
            }) => Ok(Expr::new(
                ExprKind::String(self.input.advance().kind.unwrap_string().value),
                source,
            )),
            TokenKind::OpenParen => {
                self.input.advance().kind.unwrap_open_paren();
                let inner = self.parse_expr()?;
                self.parse_token(TokenKind::CloseParen, Some("to close nested expression"))?;
                Ok(inner)
            }
            TokenKind::StructKeyword | TokenKind::UnionKeyword | TokenKind::EnumKeyword => {
                self.parse_structure_literal()
            }
            TokenKind::Identifier(_) => {
                // TODO: CLEANUP: This should be cleaned up once we have proper
                // namespaces and generic parsing that applies to all cases

                let name = self.input.eat_identifier().unwrap();
                let generics = self.parse_generics()?;

                match self.input.peek().kind {
                    TokenKind::Namespace => {
                        if !generics.is_empty() {
                            return Err(ParseErrorKind::GenericsNotSupportedHere.at(source));
                        }

                        self.parse_enum_member_literal(name, source)
                    }
                    TokenKind::OpenCurly => {
                        let peek = &self.input.peek_nth(1).kind;

                        if peek.is_extend() || peek.is_colon() {
                            let ast_type = self.parse_type_from_parts(name, generics, source)?;
                            self.parse_structure_literal_with(ast_type)
                        } else {
                            let next_three =
                                array_last::<2, 4, _>(self.input.peek_n()).map(|token| &token.kind);

                            match &next_three[..] {
                                [TokenKind::Colon, ..]
                                | [TokenKind::Identifier(_), TokenKind::Colon, ..] => {
                                    let ast_type =
                                        self.parse_type_from_parts(name, generics, source)?;
                                    self.parse_structure_literal_with(ast_type)
                                }
                                _ => Ok(Expr::new(ExprKind::Variable(name), source)),
                            }
                        }
                    }
                    TokenKind::OpenParen => self.parse_call(name, generics, source),
                    TokenKind::DeclareAssign => {
                        if !generics.is_empty() {
                            return Err(ParseErrorKind::GenericsNotSupportedHere.at(source));
                        }

                        self.parse_declare_assign(name, source)
                    }
                    _ => {
                        if !generics.is_empty() {
                            return Err(ParseErrorKind::GenericsNotSupportedHere.at(source));
                        }

                        Ok(Expr::new(ExprKind::Variable(name), source))
                    }
                }
            }
            TokenKind::Not
            | TokenKind::BitComplement
            | TokenKind::Subtract
            | TokenKind::AddressOf
            | TokenKind::Dereference => {
                let operator = match kind {
                    TokenKind::Not => UnaryOperator::Not,
                    TokenKind::BitComplement => UnaryOperator::BitComplement,
                    TokenKind::Subtract => UnaryOperator::Negate,
                    TokenKind::AddressOf => UnaryOperator::AddressOf,
                    TokenKind::Dereference => UnaryOperator::Dereference,
                    _ => unreachable!(),
                };

                // Eat unary operator
                self.input.advance();

                let inner = self.parse_expr_primary()?;

                Ok(Expr::new(
                    ExprKind::UnaryOperation(Box::new(UnaryOperation { operator, inner })),
                    source,
                ))
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

                let conditional = Conditional {
                    conditions,
                    otherwise,
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
