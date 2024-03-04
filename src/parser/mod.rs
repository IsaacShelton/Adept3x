mod error;

use std::ffi::CString;

use self::error::{ErrorInfo, ParseError};
use crate::ast::{
    self, Ast, Call, Expression, FileIdentifier, Function, Parameter, Statement, Type,
};
use crate::line_column::Location;
use crate::look_ahead::LookAhead;
use crate::token::{StringModifier, Token, TokenInfo};

struct Parser<I>
where
    I: Iterator<Item = TokenInfo>,
{
    iterator: LookAhead<I>,
    previous_location: Location,
    filename: String,
}

enum Annotation {
    Foreign,
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

    fn parse_annotation(&mut self) -> Result<Annotation, ParseError> {
        // #[annotation_name]
        // ^

        self.next();

        self.parse_token(Token::OpenBracket, Some("to begin annotation body"))?;

        let annotation_name = self.parse_identifier(Some("for annotation name"))?;
        let annotation_name_location = self.previous_location;

        self.parse_token(Token::CloseBracket, Some("to close annotation body"))?;

        match annotation_name.as_str() {
            "foreign" => Ok(Annotation::Foreign),
            _ => Err(ParseError {
                filename: Some(self.filename.clone()),
                location: Some(annotation_name_location),
                info: ErrorInfo::UnrecognizedAnnotation {
                    name: annotation_name,
                },
            }),
        }
    }

    fn parse_function(&mut self, annotations: Vec<Annotation>) -> Result<Function, ParseError> {
        // func functionName {
        //   ^

        let mut is_foreign = false;

        for annotation in annotations {
            match annotation {
                Annotation::Foreign => is_foreign = true,
            }
        }

        self.next();

        let name = self.parse_identifier(Some("after 'func' keyword"))?;
        self.ignore_newlines();

        let parameters = if self.peek_is(Token::OpenParen) {
            self.parse_function_parameters()?
        } else {
            vec![]
        };

        self.ignore_newlines();
        let return_type = self.parse_type(Some("return "), Some("for function"))?;

        let mut statements = vec![];

        if !is_foreign {
            self.ignore_newlines();
            self.parse_token(Token::OpenCurly, Some("to begin function body"))?;
            self.ignore_newlines();

            while !self.peek_is_or_eof(Token::CloseCurly) {
                statements.push(self.parse_statement()?);
                self.ignore_newlines();
            }

            self.ignore_newlines();
            self.parse_token(Token::CloseCurly, Some("to close function body"))?;
        }

        Ok(Function {
            name,
            parameters,
            return_type,
            statements,
            is_foreign,
        })
    }

    fn parse_function_parameters(&mut self) -> Result<Vec<Parameter>, ParseError> {
        // (arg1 Type1, arg2 Type2, arg3 Type3)
        // ^

        let mut parameters = vec![];

        self.parse_token(Token::OpenParen, Some("to begin function parameters"))?;
        self.ignore_newlines();

        while !self.peek_is_or_eof(Token::CloseParen) {
            // Parse argument

            if !parameters.is_empty() {
                self.parse_token(Token::Comma, Some("after parameter"))?;
                self.ignore_newlines();
            }

            let name = self.parse_identifier(Some("for parameter name"))?;
            self.ignore_newlines();
            let ast_type = self.parse_type(None::<&str>, Some("for parameter"))?;
            self.ignore_newlines();

            parameters.push(Parameter { name, ast_type });
        }

        self.parse_token(Token::CloseParen, Some("to end function parameters"))?;
        Ok(parameters)
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.peek() {
            Some(TokenInfo {
                token: Token::ReturnKeyword,
                ..
            }) => self.parse_return(),
            None => Err(ParseError {
                filename: Some(self.filename.clone()),
                location: Some(self.previous_location),
                info: ErrorInfo::UnexpectedToken { unexpected: None },
            }),
            _ => Ok(Statement::Expression(self.parse_expression()?)),
        }
    }

    fn parse_return(&mut self) -> Result<Statement, ParseError> {
        // return VALUE
        //          ^

        self.parse_token(Token::ReturnKeyword, Some("for return statement"))?;

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
            Some(TokenInfo {
                token:
                    Token::String {
                        value,
                        modifier: StringModifier::NullTerminated,
                    },
                ..
            }) => Ok(Expression::NullTerminatedString(
                CString::new(value).expect("valid null-terminated string"),
            )),
            Some(TokenInfo {
                token: Token::Identifier(identifier),
                ..
            }) => {
                if self.peek_is(Token::OpenParen) {
                    self.parse_call(identifier)
                } else {
                    Ok(Expression::Variable(identifier))
                }
            }
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

    fn parse_call(&mut self, function_name: String) -> Result<Expression, ParseError> {
        // function_name(arg1, arg2, arg3)
        //              ^

        let mut arguments = vec![];

        self.parse_token(Token::OpenParen, Some("to begin call argument list"))?;
        self.ignore_newlines();

        while !self.peek_is_or_eof(Token::CloseParen) {
            if !arguments.is_empty() {
                self.parse_token(Token::Comma, Some("to separate arguments"))?;
            }

            arguments.push(self.parse_expression()?);
            self.ignore_newlines();
        }

        self.parse_token(Token::CloseParen, Some("to end call argument list"))?;

        Ok(Expression::Call(Call {
            function_name,
            arguments,
        }))
    }

    fn parse_type(
        &mut self,
        prefix: Option<impl ToString>,
        for_reason: Option<impl ToString>,
    ) -> Result<Type, ParseError> {
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
                    "ptr" => {
                        if self.peek_is(Token::Colon) {
                            self.parse_token(Token::Colon, None::<&str>)?;
                            let inner = self.parse_type(None::<&str>, None::<&str>)?;
                            Ok(Type::Pointer(Box::new(inner)))
                        } else {
                            Ok(Type::Pointer(Box::new(Type::Void)))
                        }
                    }
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
                    prefix: prefix.map(|prefix| prefix.to_string()),
                    for_reason: for_reason.map(|for_reason| for_reason.to_string()),
                    got: Some(format!("{}", token)),
                },
            }),
            None => Err(ParseError {
                filename: Some(self.filename.clone()),
                location: Some(self.previous_location),
                info: ErrorInfo::ExpectedType {
                    prefix: prefix.map(|prefix| prefix.to_string()),
                    for_reason: for_reason.map(|for_reason| for_reason.to_string()),
                    got: None,
                },
            }),
        }
    }
}

pub fn parse(tokens: impl Iterator<Item = TokenInfo>, filename: String) -> Result<Ast, ParseError> {
    let mut parser = Parser::new(tokens, filename.clone());
    let mut ast = Ast::new(filename.clone());

    let mut file = ast.new_file(FileIdentifier::Local(filename));
    let mut annotations = Vec::new();

    while let Some(token_info) = parser.peek() {
        match token_info.token {
            Token::FuncKeyword => {
                let function = parser
                    .parse_function(std::mem::replace(&mut annotations, Default::default()))?;
                file.functions.push(function);
            }
            Token::Hash => {
                annotations.push(parser.parse_annotation()?);
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
