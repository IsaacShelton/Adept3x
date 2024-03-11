mod annotation;
mod error;
mod input;
mod make_error;

use self::annotation::Annotation;
use self::error::{ErrorInfo, ParseError};
use self::input::Input;
use crate::ast::{
    self, Ast, Call, DeclareAssign, Expression, File, FileIdentifier, Function, Parameter,
    Parameters, Statement, Type,
};
use crate::line_column::Location;
use crate::look_ahead::LookAhead;
use crate::token::{StringModifier, Token, TokenInfo};
use std::borrow::Borrow;
use std::ffi::CString;
use std::fmt::Display;

struct Parser<I>
where
    I: Iterator<Item = TokenInfo>,
{
    input: Input<I>,
}

impl<I> Parser<I>
where
    I: Iterator<Item = TokenInfo>,
{
    pub fn new(input: Input<I>) -> Self {
        Self { input }
    }

    fn parse(mut self) -> Result<Ast, ParseError> {
        let source_filename = self.input.filename();
        let mut ast = Ast::new(source_filename.into());
        let ast_file = ast.new_file(FileIdentifier::Local(source_filename.into()));

        while !self.input.peek().is_end_of_file() {
            self.parse_top_level(ast_file)?;
        }

        Ok(ast)
    }

    pub fn parse_top_level(&mut self, ast_file: &mut File) -> Result<(), ParseError> {
        let mut annotations = Vec::new();

        // Ignore preceeding newlines
        self.ignore_newlines();

        // Parse annotations
        while self.input.peek().is_hash() {
            annotations.push(self.parse_annotation()?);
            self.ignore_newlines();
        }

        // Ignore newlines after annotations
        self.ignore_newlines();

        // Parse top-level construct
        match self.input.peek().token {
            Token::FuncKeyword => {
                ast_file.functions.push(self.parse_function(annotations)?);
            }
            Token::EndOfFile => {
                if annotations.len() > 0 {
                    let info = self.input.advance();
                    return Err(self.expected_top_level_construct(&info));
                }
            }
            _ => {
                return Err(self.unexpected_token_is_next());
            }
        }

        Ok(())
    }

    pub fn parse_token(
        &mut self,
        expected_token: impl Borrow<Token>,
        for_reason: Option<impl ToString>,
    ) -> Result<(), ParseError> {
        let info = self.input.advance();
        let expected_token = expected_token.borrow();

        if info.token == *expected_token {
            return Ok(());
        }

        Err(self.expected_token(expected_token, for_reason, info))
    }

    pub fn parse_identifier(
        &mut self,
        for_reason: Option<impl ToString>,
    ) -> Result<String, ParseError> {
        Ok(self.parse_identifier_keep_location(for_reason)?.0)
    }

    pub fn parse_identifier_keep_location(
        &mut self,
        for_reason: Option<impl ToString>,
    ) -> Result<(String, Location), ParseError> {
        let info = self.input.advance();

        if let Token::Identifier(identifier) = &info.token {
            Ok((identifier.into(), info.location))
        } else {
            Err(self.expected_token("identifier", for_reason, info))
        }
    }

    pub fn ignore_newlines(&mut self) {
        while let Token::Newline = self.input.peek().token {
            self.input.advance();
        }
    }

    fn parse_annotation(&mut self) -> Result<Annotation, ParseError> {
        // #[annotation_name]
        // ^

        self.parse_token(Token::Hash, Some("to begin annotation"))?;
        self.parse_token(Token::OpenBracket, Some("to begin annotation body"))?;

        let (annotation_name, location) =
            self.parse_identifier_keep_location(Some("for annotation name"))?;

        self.parse_token(Token::CloseBracket, Some("to close annotation body"))?;

        match annotation_name.as_str() {
            "foreign" => Ok(Annotation::Foreign),
            _ => Err(ParseError {
                filename: Some(self.input.filename().to_string()),
                location: Some(location),
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

        self.input.advance();

        let name = self.parse_identifier(Some("after 'func' keyword"))?;
        self.ignore_newlines();

        let parameters = if self.input.peek_is(Token::OpenParen) {
            self.parse_function_parameters()?
        } else {
            Parameters::default()
        };

        self.ignore_newlines();

        let return_type = if self.input.peek_is(Token::OpenCurly) {
            ast::Type::Void
        } else {
            self.parse_type(Some("return "), Some("for function"))?
        };

        let mut statements = vec![];

        if !is_foreign {
            self.ignore_newlines();
            self.parse_token(Token::OpenCurly, Some("to begin function body"))?;
            self.ignore_newlines();

            while !self.input.peek_is_or_eof(Token::CloseCurly) {
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

    fn parse_function_parameters(&mut self) -> Result<Parameters, ParseError> {
        // (arg1 Type1, arg2 Type2, arg3 Type3)
        // ^

        let mut required = vec![];
        let mut is_cstyle_vararg = false;

        self.parse_token(Token::OpenParen, Some("to begin function parameters"))?;
        self.ignore_newlines();

        while !self.input.peek_is_or_eof(Token::CloseParen) {
            // Parse argument

            if !required.is_empty() {
                self.parse_token(Token::Comma, Some("after parameter"))?;
                self.ignore_newlines();
            }

            if self.input.peek_is(Token::Ellipsis) {
                is_cstyle_vararg = true;
                self.input.advance();
                self.ignore_newlines();
                break;
            }

            let name = self.parse_identifier(Some("for parameter name"))?;
            self.ignore_newlines();
            let ast_type = self.parse_type(None::<&str>, Some("for parameter"))?;
            self.ignore_newlines();
            required.push(Parameter { name, ast_type });
        }

        self.parse_token(Token::CloseParen, Some("to end function parameters"))?;

        Ok(Parameters {
            required,
            is_cstyle_vararg,
        })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        match self.input.peek().token {
            Token::ReturnKeyword => self.parse_return(),
            Token::EndOfFile => Err(self.unexpected_token_is_next()),
            _ => Ok(Statement::Expression(self.parse_expression()?)),
        }
    }

    fn parse_return(&mut self) -> Result<Statement, ParseError> {
        // return VALUE
        //          ^

        self.parse_token(Token::ReturnKeyword, Some("for return statement"))?;

        if self.input.peek_is(Token::Newline) {
            Ok(Statement::Return(None))
        } else {
            Ok(Statement::Return(Some(self.parse_expression()?)))
        }
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        match self.input.advance() {
            TokenInfo {
                token: Token::Integer { value },
                ..
            } => Ok(Expression::Integer(value)),
            TokenInfo {
                token:
                    Token::String {
                        value,
                        modifier: StringModifier::NullTerminated,
                    },
                ..
            } => Ok(Expression::NullTerminatedString(
                CString::new(value).expect("valid null-terminated string"),
            )),
            TokenInfo {
                token: Token::Identifier(identifier),
                ..
            } => {
                if self.input.peek_is(Token::OpenParen) {
                    self.parse_call(identifier)
                } else if self.input.peek_is(Token::DeclareAssign) {
                    self.parse_declare_assign(identifier)
                } else {
                    Ok(Expression::Variable(identifier))
                }
            }
            unexpected => Err(ParseError {
                filename: Some(self.input.filename().to_string()),
                location: Some(unexpected.location),
                info: ErrorInfo::UnexpectedToken {
                    unexpected: unexpected.to_string(),
                },
            }),
        }
    }

    fn parse_call(&mut self, function_name: String) -> Result<Expression, ParseError> {
        // function_name(arg1, arg2, arg3)
        //              ^

        let mut arguments = vec![];

        self.parse_token(Token::OpenParen, Some("to begin call argument list"))?;
        self.ignore_newlines();

        while !self.input.peek_is_or_eof(Token::CloseParen) {
            if !arguments.is_empty() {
                self.parse_token(Token::Comma, Some("to separate arguments"))?;
                self.ignore_newlines();
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

    fn parse_declare_assign(&mut self, variable_name: String) -> Result<Expression, ParseError> {
        // variable_name := value
        //               ^

        self.parse_token(
            Token::DeclareAssign,
            Some("for variable declaration assignment"),
        )?;
        self.ignore_newlines();

        let value = self.parse_expression()?;

        Ok(Expression::DeclareAssign(DeclareAssign {
            name: variable_name,
            value: Box::new(value),
        }))
    }

    fn parse_type(
        &mut self,
        prefix: Option<impl ToString>,
        for_reason: Option<impl ToString>,
    ) -> Result<Type, ParseError> {
        match self.input.advance() {
            TokenInfo {
                token: Token::Identifier(identifier),
                location,
            } => {
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
                        if self.input.peek_is(Token::Colon) {
                            self.parse_token(Token::Colon, None::<&str>)?;
                            let inner = self.parse_type(None::<&str>, None::<&str>)?;
                            Ok(Type::Pointer(Box::new(inner)))
                        } else {
                            Ok(Type::Pointer(Box::new(Type::Void)))
                        }
                    }
                    _ => Err(ParseError {
                        filename: Some(self.input.filename().to_string()),
                        location: Some(location),
                        info: ErrorInfo::UndeclaredType { name: identifier },
                    }),
                }
            }
            unexpected => Err(ParseError {
                filename: Some(self.input.filename().to_string()),
                location: Some(unexpected.location),
                info: ErrorInfo::ExpectedType {
                    prefix: prefix.map(|prefix| prefix.to_string()),
                    for_reason: for_reason.map(|for_reason| for_reason.to_string()),
                    got: unexpected.to_string(),
                },
            }),
        }
    }
}

pub fn parse(tokens: impl Iterator<Item = TokenInfo>, filename: String) -> Result<Ast, ParseError> {
    Parser::new(Input::new(tokens, filename.clone())).parse()
}
