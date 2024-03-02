mod error;

use self::error::{ErrorInfo, ParseError};
use crate::ast::{self, Ast, Expression, Function, Statement, Type};
use crate::line_column::Location;
use crate::look_ahead::LookAhead;
use crate::token::{Token, TokenInfo};

struct Parser<I>
where
    I: Iterator<Item = TokenInfo>,
{
    iterator: LookAhead<I>,
    previous_location: Location,
    filename: String,
}

impl<I> Parser<I>
where
    I: Iterator<Item = TokenInfo>,
{
    pub fn new(iterator: I, filename: String) -> Self {
        Self {
            iterator: LookAhead::new(iterator),
            previous_location: Location::new(1, 1),
            filename,
        }
    }

    pub fn peek(&mut self) -> Option<&TokenInfo> {
        self.iterator.peek()
    }

    #[allow(dead_code)]
    pub fn peek_nth(&mut self, n: usize) -> Option<&TokenInfo> {
        self.iterator.peek_nth(n)
    }

    pub fn peek_is(&mut self, token: Token) -> bool {
        if let Some(token_info) = self.iterator.peek() {
            token_info.token == token
        } else {
            false
        }
    }

    pub fn peek_is_or_eof(&mut self, token: Token) -> bool {
        if let Some(token_info) = self.iterator.peek() {
            token_info.token == token
        } else {
            true
        }
    }

    pub fn next(&mut self) -> Option<TokenInfo> {
        self.iterator.next().map(|token_info| {
            self.previous_location = token_info.location;
            token_info
        })
    }

    pub fn parse_identifier(
        &mut self,
        for_reason: Option<impl ToString>,
    ) -> Result<String, ParseError> {
        if let Some(token_info) = self.next() {
            if let Token::Identifier(identifier) = token_info.token {
                Ok(identifier)
            } else {
                Err(ParseError {
                    filename: Some(self.filename.clone()),
                    location: Some(token_info.location),
                    info: ErrorInfo::Expected {
                        expected: format!("identifier"),
                        for_reason: for_reason.map(|reason| reason.to_string()),
                        got: Some(format!("{}", token_info.token)),
                    },
                })
            }
        } else {
            Err(ParseError {
                filename: Some(self.filename.clone()),
                location: Some(self.previous_location),
                info: ErrorInfo::Expected {
                    expected: format!("identifier"),
                    for_reason: for_reason.map(|reason| reason.to_string()),
                    got: None,
                },
            })
        }
    }

    pub fn parse_token(
        &mut self,
        token: Token,
        for_reason: Option<impl ToString>,
    ) -> Result<(), ParseError> {
        let token_info = self.next();

        if let Some(token_info) = &token_info {
            if token_info.token == token {
                return Ok(());
            }
        }

        Err(self.error_expected_token(token, for_reason, token_info))
    }

    pub fn error_expected_token(
        &self,
        token: Token,
        for_reason: Option<impl ToString>,
        token_info: Option<TokenInfo>,
    ) -> ParseError {
        if let Some(token_info) = token_info {
            ParseError {
                filename: Some(self.filename.clone()),
                location: Some(token_info.location),
                info: ErrorInfo::Expected {
                    expected: format!("{}", token),
                    got: Some(format!("{}", token_info.token)),
                    for_reason: for_reason.map(|reason| reason.to_string()),
                },
            }
        } else {
            ParseError {
                filename: Some(self.filename.clone()),
                location: Some(self.previous_location),
                info: ErrorInfo::Expected {
                    expected: format!("{}", token),
                    got: None,
                    for_reason: for_reason.map(|reason| reason.to_string()),
                },
            }
        }
    }

    pub fn ignore_newlines(&mut self) {
        while let Some(TokenInfo {
            token: Token::Newline,
            ..
        }) = self.peek()
        {
            self.next();
        }
    }

    pub fn error_unexpected_token_is_next(&mut self) -> ParseError {
        let token_info = self.next();
        self.error_unexpected_token(token_info)
    }

    pub fn error_unexpected_token(&self, token_info: Option<TokenInfo>) -> ParseError {
        if let Some(token_info) = token_info {
            ParseError {
                filename: Some(self.filename.clone()),
                location: Some(token_info.location),
                info: ErrorInfo::UnexpectedToken {
                    unexpected: Some(format!("{}", token_info.token)),
                },
            }
        } else {
            ParseError {
                filename: Some(self.filename.clone()),
                location: Some(self.previous_location),
                info: ErrorInfo::UnexpectedToken { unexpected: None },
            }
        }
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        // func functionName {
        //           ^

        let name = self.parse_identifier(Some("after 'func' keyword"))?;
        let mut parameters = vec![];
        let mut statements = vec![];

        self.ignore_newlines();
        let return_type = self.parse_type()?;

        self.ignore_newlines();
        self.parse_token(Token::OpenCurly, Some("to begin function body"))?;
        self.ignore_newlines();

        while !self.peek_is_or_eof(Token::CloseCurly) {
            statements.push(self.parse_statement()?);
        }

        self.ignore_newlines();
        self.parse_token(Token::CloseCurly, Some("to close function body"))?;

        Ok(Function {
            name,
            parameters,
            return_type,
            statements,
        })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.next() {
            Some(TokenInfo {
                token: Token::ReturnKeyword,
                ..
            }) => self.parse_return(),
            Some(TokenInfo { token, location }) => Err(ParseError {
                filename: Some(self.filename.clone()),
                location: Some(location),
                info: ErrorInfo::UnexpectedToken {
                    unexpected: Some(format!("{}", token)),
                },
            }),
            None => Err(ParseError {
                filename: Some(self.filename.clone()),
                location: Some(self.previous_location),
                info: ErrorInfo::UnexpectedToken { unexpected: None },
            }),
        }
    }

    fn parse_return(&mut self) -> Result<Statement, ParseError> {
        // return VALUE
        //          ^

        if self.peek_is(Token::Newline) {
            Ok(Statement::Return(None))
        } else {
            Ok(Statement::Return(Some(self.parse_expression()?)))
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        match self.next() {
            Some(TokenInfo {
                token: Token::Integer { value },
                ..
            }) => Ok(Expression::Integer(value)),
            Some(TokenInfo { token, location }) => Err(ParseError {
                filename: Some(self.filename.clone()),
                location: Some(location),
                info: ErrorInfo::UnexpectedToken {
                    unexpected: Some(format!("{}", token)),
                },
            }),
            None => Err(ParseError {
                filename: Some(self.filename.clone()),
                location: Some(self.previous_location),
                info: ErrorInfo::UnexpectedToken { unexpected: None },
            }),
        }
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        match self.next() {
            Some(TokenInfo {
                token: Token::Identifier(identifier),
                location,
            }) => {
                use ast::{IntegerBits::*, IntegerSign::*};

                match identifier.as_str() {
                    "int" => Ok(Type::Integer {
                        bits: Normal,
                        sign: Signed,
                    }),
                    "uint" => Ok(Type::Integer {
                        bits: Normal,
                        sign: Unsigned,
                    }),
                    "int8" => Ok(Type::Integer {
                        bits: Bits8,
                        sign: Signed,
                    }),
                    "uint8" => Ok(Type::Integer {
                        bits: Bits8,
                        sign: Unsigned,
                    }),
                    "int16" => Ok(Type::Integer {
                        bits: Bits16,
                        sign: Signed,
                    }),
                    "uint16" => Ok(Type::Integer {
                        bits: Bits16,
                        sign: Unsigned,
                    }),
                    "int32" => Ok(Type::Integer {
                        bits: Bits32,
                        sign: Signed,
                    }),
                    "uint32" => Ok(Type::Integer {
                        bits: Bits32,
                        sign: Unsigned,
                    }),
                    "int64" => Ok(Type::Integer {
                        bits: Bits64,
                        sign: Signed,
                    }),
                    "uint64" => Ok(Type::Integer {
                        bits: Bits64,
                        sign: Unsigned,
                    }),
                    "void" => Ok(Type::Void),
                    _ => Err(ParseError {
                        filename: Some(self.filename.clone()),
                        location: Some(location),
                        info: ErrorInfo::UndeclaredType { name: identifier },
                    }),
                }
            }
            Some(TokenInfo { location, token }) => Err(ParseError {
                filename: Some(self.filename.clone()),
                location: Some(location),
                info: ErrorInfo::ExpectedType {
                    prefix: Some("return ".into()),
                    for_reason: Some("for function".into()),
                    got: Some(format!("{}", token)),
                },
            }),
            None => Err(ParseError {
                filename: Some(self.filename.clone()),
                location: Some(self.previous_location),
                info: ErrorInfo::ExpectedType {
                    prefix: Some("return ".into()),
                    for_reason: Some("for function".into()),
                    got: None,
                },
            }),
        }
    }
}

pub fn parse(tokens: impl Iterator<Item = TokenInfo>, filename: String) -> Result<Ast, ParseError> {
    let mut parser = Parser::new(tokens, filename);
    let mut ast = Ast::new();

    while let Some(token_info) = parser.peek() {
        match token_info.token {
            Token::FuncKeyword => {
                parser.next();
                ast.functions.push(parser.parse_function()?);
            }
            Token::Newline => {
                parser.next();
            }
            _ => {
                return Err(parser.error_unexpected_token_is_next());
            }
        }
    }

    Ok(ast)
}
