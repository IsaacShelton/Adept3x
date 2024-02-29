use crate::ast::{Ast, Function};
use crate::line_column::Location;
use crate::look_ahead::LookAhead;
use crate::token::{Token, TokenInfo};

#[derive(Clone, Debug)]
pub struct ParseError {
    pub message: String,
    pub location: Location,
}

struct Parser<I>
where
    I: Iterator<Item = TokenInfo>,
{
    iterator: LookAhead<I>,
    previous_location: Location,
}

impl<I> Parser<I>
where
    I: Iterator<Item = TokenInfo>,
{
    pub fn new(iterator: I) -> Self {
        Self {
            iterator: LookAhead::new(iterator),
            previous_location: Location::new(1, 1),
        }
    }

    #[allow(dead_code)]
    pub fn peek(&mut self) -> Option<&TokenInfo> {
        self.iterator.peek()
    }

    #[allow(dead_code)]
    pub fn peek_nth(&mut self, n: usize) -> Option<&TokenInfo> {
        self.iterator.peek_nth(n)
    }

    #[allow(dead_code)]
    pub fn next(&mut self) -> Option<TokenInfo> {
        self.iterator.next().map(|token_info| {
            self.previous_location = token_info.location;
            token_info
        })
    }

    pub fn parse_identifier(&mut self, for_reason: Option<&str>) -> Result<String, ParseError> {
        if let Some(token_info) = self.next() {
            if let Token::Identifier(identifier) = token_info.token {
                Ok(identifier)
            } else {
                Err(ParseError {
                    message: if let Some(for_reason) = for_reason {
                        format!(
                            "Expected identifier {}, instead got {}",
                            for_reason, token_info.token
                        )
                    } else {
                        format!("Expected identifier, instead got {}", token_info.token)
                    },
                    location: token_info.location,
                })
            }
        } else {
            Err(ParseError {
                message: if let Some(for_reason) = for_reason {
                    format!("Expected identifier {}, instead got end-of-file", for_reason)
                } else {
                    "Expected identifier, instead got end-of-file".into()
                },
                location: self.previous_location,
            })
        }
    }

    pub fn error_unexpected_token_is_next(&mut self) -> ParseError {
        let token_info = self.next();
        self.error_unexpected_token(token_info)
    }

    pub fn error_unexpected_token(&self, token_info: Option<TokenInfo>) -> ParseError {
        if let Some(token_info) = token_info {
            ParseError {
                message: format!("Unexpected token {}", token_info.token),
                location: token_info.location,
            }
        } else {
            ParseError {
                message: "Unexpected end-of-file".into(),
                location: self.previous_location,
            }
        }
    }

    fn parse_function(&mut self) -> Result<Function, ParseError> {
        // func functionName {
        //           ^

        let name = self.parse_identifier(Some("after 'func' keyword"))?;
        Ok(Function { name })
    }
}

pub fn parse(tokens: impl Iterator<Item = TokenInfo>) -> Result<Ast, ParseError> {
    let mut parser = Parser::new(tokens);
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
