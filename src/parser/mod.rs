mod error;

use crate::ast::{Ast, Function};
use crate::line_column::Location;
use crate::look_ahead::LookAhead;
use crate::token::{Token, TokenInfo};
use self::error::{ErrorInfo, ParseError};

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

    pub fn next(&mut self) -> Option<TokenInfo> {
        self.iterator.next().map(|token_info| {
            self.previous_location = token_info.location;
            token_info
        })
    }

    pub fn parse_identifier(&mut self, for_reason: Option<impl ToString>) -> Result<String, ParseError> {
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
        if let Some(token_info) = self.next() {
            if token_info.token == token {
                return Ok(());
            }

            Err(ParseError {
                filename: Some(self.filename.clone()),
                location: Some(token_info.location),
                info: ErrorInfo::Expected {
                    expected: format!("{}", token),
                    got: Some(format!("{}", token_info.token)),
                    for_reason: for_reason.map(|reason| reason.to_string()),
                }
            })
        } else {
            Err(ParseError {
                filename: Some(self.filename.clone()),
                location: Some(self.previous_location),
                info: ErrorInfo::Expected {
                    expected: format!("{}", token),
                    got: None,
                    for_reason: for_reason.map(|reason| reason.to_string()),
                }
            })
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
                }
            }
        } else {
            ParseError {
                filename: Some(self.filename.clone()),
                location: Some(self.previous_location),
                info: ErrorInfo::UnexpectedToken {
                    unexpected: None,
                }
            }
        }
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        // func functionName {
        //           ^

        let name = self.parse_identifier(Some("after 'func' keyword"))?;
        self.ignore_newlines();
        self.parse_token(Token::OpenCurly, Some("to begin function body"))?;
        self.ignore_newlines();
        self.parse_token(Token::CloseCurly, Some("to close function body"))?;

        let parameters = vec![];
        let statements = vec![];

        Ok(Function {
            name,
            parameters,
            statements,
        })
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
