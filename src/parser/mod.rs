mod annotation;
mod error;
mod input;
mod make_error;

use self::annotation::{Annotation, AnnotationKind};
use self::error::{ParseError, ParseErrorKind};
use self::input::Input;
use crate::ast::{
    self, Ast, BinaryOperation, Call, DeclareAssign, Expression, ExpressionKind, File, FileIdentifier, Function, Global, Parameter, Parameters, Source, Statement, StatementKind, Type
};
use crate::line_column::Location;
use crate::look_ahead::LookAhead;
use crate::source_file_cache::{self, SourceFileCache, SourceFileCacheKey};
use crate::token::{StringModifier, Token, TokenKind};
use ast::BinaryOperator;
use std::borrow::Borrow;
use std::ffi::CString;
use std::fmt::Display;

struct Parser<'a, I>
where
    I: Iterator<Item = Token>,
{
    input: Input<'a, I>,
}

impl<'a, I> Parser<'a, I>
where
    I: Iterator<Item = Token>,
{
    pub fn new(input: Input<'a, I>) -> Self {
        Self { input }
    }

    fn parse(mut self) -> Result<Ast<'a>, ParseError> {
        // Get primary filename
        let filename = self.input.filename();

        // Create global ast
        let mut ast = Ast::new(filename.into(), self.input.source_file_cache());

        // Parse primary file
        self.parse_into(&mut ast, filename.into())?;

        // Return global ast
        Ok(ast)
    }

    fn parse_into(&mut self, ast: &mut Ast, filename: String) -> Result<(), ParseError> {
        // Create ast file
        let ast_file = ast.new_file(FileIdentifier::Local(filename));

        while !self.input.peek().is_end_of_file() {
            self.parse_top_level(ast_file)?;
        }

        Ok(())
    }

    fn parse_top_level(&mut self, ast_file: &mut File) -> Result<(), ParseError> {
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
        match self.input.peek().kind {
            TokenKind::FuncKeyword => {
                ast_file.functions.push(self.parse_function(annotations)?);
            }
            TokenKind::Identifier(_) => {
                ast_file.globals.push(self.parse_global(annotations)?);
            }
            TokenKind::EndOfFile => {
                // End-of-file is only okay if no preceeding annotations
                if annotations.len() > 0 {
                    let token = self.input.advance();
                    return Err(self.expected_top_level_construct(&token));
                }
            }
            _ => {
                return Err(self.unexpected_token_is_next());
            }
        }

        Ok(())
    }

    fn parse_token(
        &mut self,
        expected_token: impl Borrow<TokenKind>,
        for_reason: Option<impl ToString>,
    ) -> Result<Location, ParseError> {
        let token = self.input.advance();
        let expected_token = expected_token.borrow();

        if token.kind == *expected_token {
            return Ok(token.location);
        }

        Err(self.expected_token(expected_token, for_reason, token))
    }

    fn parse_identifier(
        &mut self,
        for_reason: Option<impl ToString>,
    ) -> Result<String, ParseError> {
        Ok(self.parse_identifier_keep_location(for_reason)?.0)
    }

    fn parse_identifier_keep_location(
        &mut self,
        for_reason: Option<impl ToString>,
    ) -> Result<(String, Location), ParseError> {
        let token = self.input.advance();

        if let TokenKind::Identifier(identifier) = &token.kind {
            Ok((identifier.into(), token.location))
        } else {
            Err(self.expected_token("identifier", for_reason, token))
        }
    }

    fn ignore_newlines(&mut self) {
        while let TokenKind::Newline = self.input.peek().kind {
            self.input.advance();
        }
    }

    fn parse_annotation(&mut self) -> Result<Annotation, ParseError> {
        // #[annotation_name]
        // ^

        self.parse_token(TokenKind::Hash, Some("to begin annotation"))?;
        self.parse_token(TokenKind::OpenBracket, Some("to begin annotation body"))?;

        let (annotation_name, location) =
            self.parse_identifier_keep_location(Some("for annotation name"))?;

        self.parse_token(TokenKind::CloseBracket, Some("to close annotation body"))?;

        match annotation_name.as_str() {
            "foreign" => Ok(Annotation::new(AnnotationKind::Foreign, location)),
            "thread_local" => Ok(Annotation::new(AnnotationKind::ThreadLocal, location)),
            _ => Err(ParseError {
                filename: Some(self.input.filename().to_string()),
                location: Some(location),
                kind: ParseErrorKind::UnrecognizedAnnotation {
                    name: annotation_name,
                },
            }),
        }
    }

    fn parse_global(&mut self, annotations: Vec<Annotation>) -> Result<Global, ParseError> {
        // my_global_name Type
        //      ^

        let mut is_foreign = false;
        let mut is_thread_local = false;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Foreign => is_foreign = true,
                AnnotationKind::ThreadLocal => is_thread_local = true,
            }
        }

        let (name, location) = self.parse_identifier_keep_location(Some("for name of global variable"))?;
        let ast_type = self.parse_type(None::<&str>, Some("for type of global variable"))?;

        Ok(Global {
            name,
            ast_type,
            source: self.source(location),
            is_foreign,
            is_thread_local,
        })
    }

    fn parse_function(&mut self, annotations: Vec<Annotation>) -> Result<Function, ParseError> {
        // func functionName {
        //   ^

        let mut is_foreign = false;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Foreign => is_foreign = true,
                _ => return Err(self.unexpected_annotation(annotation.kind.to_string(), annotation.location, Some("for function"))),
            }
        }

        self.input.advance();

        let name = self.parse_identifier(Some("after 'func' keyword"))?;
        self.ignore_newlines();

        let parameters = if self.input.peek_is(TokenKind::OpenParen) {
            self.parse_function_parameters()?
        } else {
            Parameters::default()
        };

        self.ignore_newlines();

        let return_type = if self.input.peek_is(TokenKind::OpenCurly) {
            ast::Type::Void
        } else {
            self.parse_type(Some("return "), Some("for function"))?
        };

        let mut statements = vec![];

        if !is_foreign {
            self.ignore_newlines();
            self.parse_token(TokenKind::OpenCurly, Some("to begin function body"))?;
            self.ignore_newlines();

            while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
                statements.push(self.parse_statement()?);
                self.ignore_newlines();
            }

            self.ignore_newlines();
            self.parse_token(TokenKind::CloseCurly, Some("to close function body"))?;
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

        self.parse_token(TokenKind::OpenParen, Some("to begin function parameters"))?;
        self.ignore_newlines();

        while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
            // Parse argument

            if !required.is_empty() {
                self.parse_token(TokenKind::Comma, Some("after parameter"))?;
                self.ignore_newlines();
            }

            if self.input.peek_is(TokenKind::Ellipsis) {
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

        self.parse_token(TokenKind::CloseParen, Some("to end function parameters"))?;

        Ok(Parameters {
            required,
            is_cstyle_vararg,
        })
    }

    fn parse_statement(&mut self) -> Result<Statement, ParseError> {
        let location = self.input.peek().location;

        match self.input.peek().kind {
            TokenKind::ReturnKeyword => self.parse_return(),
            TokenKind::EndOfFile => Err(self.unexpected_token_is_next()),
            _ => Ok(Statement::new(
                StatementKind::Expression(self.parse_expression()?),
                self.source(location),
            )),
        }
    }

    fn parse_return(&mut self) -> Result<Statement, ParseError> {
        // return VALUE
        //          ^

        let location = self.parse_token(TokenKind::ReturnKeyword, Some("for return statement"))?;

        Ok(Statement::new(
            StatementKind::Return(if self.input.peek_is(TokenKind::Newline) {
                None
            } else {
                Some(self.parse_expression()?)
            }),
            self.source(location),
        ))
    }

    fn parse_expression(&mut self) -> Result<Expression, ParseError> {
        let primary = self.parse_expression_primary()?;
        self.parse_operator_expression(0, primary)
    }

    fn parse_operator_expression(
        &mut self,
        precedence: usize,
        expression: Expression,
    ) -> Result<Expression, ParseError> {
        let mut lhs = expression;

        loop {
            let operator = self.input.peek();
            let location = operator.location;
            let next_precedence = operator.kind.precedence();

            if (is_terminating_token(&operator.kind)
                || (next_precedence + is_right_associative(operator) as usize) < precedence)
            {
                return Ok(lhs);
            }

            let binary_operator = match operator.kind {
                TokenKind::Add => BinaryOperator::Add,
                TokenKind::Subtract => BinaryOperator::Subtract,
                TokenKind::Multiply => BinaryOperator::Multiply,
                TokenKind::Divide => BinaryOperator::Divide,
                TokenKind::Modulus => BinaryOperator::Modulus,
                _ => return Ok(lhs),
            };

            lhs = self.parse_math(lhs, binary_operator, next_precedence, location)?;
        }
    }

    fn parse_expression_primary(&mut self) -> Result<Expression, ParseError> {
        let expression = self.parse_expression_primary_base()?;
        self.parse_expression_primary_post(expression)
    }

    fn parse_expression_primary_base(&mut self) -> Result<Expression, ParseError> {
        match self.input.advance() {
            Token {
                kind: TokenKind::Integer { value },
                location,
            } => Ok(Expression::new(
                ExpressionKind::Integer(value),
                self.source(location),
            )),
            Token {
                kind:
                    TokenKind::String {
                        value,
                        modifier: StringModifier::NullTerminated,
                    },
                location,
            } => Ok(Expression::new(
                ExpressionKind::NullTerminatedString(
                    CString::new(value).expect("valid null-terminated string"),
                ),
                self.source(location),
            )),
            Token {
                kind: TokenKind::Identifier(identifier),
                location,
            } => {
                if self.input.peek_is(TokenKind::OpenParen) {
                    self.parse_call(identifier, self.source(location))
                } else if self.input.peek_is(TokenKind::DeclareAssign) {
                    self.parse_declare_assign(identifier, self.source(location))
                } else {
                    Ok(Expression::new(
                        ExpressionKind::Variable(identifier),
                        self.source(location),
                    ))
                }
            }
            unexpected => Err(ParseError {
                filename: Some(self.input.filename().to_string()),
                location: Some(unexpected.location),
                kind: ParseErrorKind::UnexpectedToken {
                    unexpected: unexpected.to_string(),
                },
            }),
        }
    }

    fn parse_expression_primary_post(
        &mut self,
        base: Expression,
    ) -> Result<Expression, ParseError> {
        Ok(base)
    }

    fn parse_call(
        &mut self,
        function_name: String,
        source: Source,
    ) -> Result<Expression, ParseError> {
        // function_name(arg1, arg2, arg3)
        //              ^

        let mut arguments = vec![];

        self.parse_token(TokenKind::OpenParen, Some("to begin call argument list"))?;
        self.ignore_newlines();

        while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
            if !arguments.is_empty() {
                self.parse_token(TokenKind::Comma, Some("to separate arguments"))?;
                self.ignore_newlines();
            }

            arguments.push(self.parse_expression()?);
            self.ignore_newlines();
        }

        self.parse_token(TokenKind::CloseParen, Some("to end call argument list"))?;

        Ok(Expression::new(
            ExpressionKind::Call(Call {
                function_name,
                arguments,
            }),
            source,
        ))
    }

    fn parse_math(
        &mut self,
        lhs: Expression,
        operator: BinaryOperator,
        operator_precedence: usize,
        location: Location,
    ) -> Result<Expression, ParseError> {
        let rhs = self.parse_math_rhs(operator_precedence)?;

        Ok(Expression::new(
            ExpressionKind::BinaryOperation(Box::new(BinaryOperation {
                operator,
                left: lhs,
                right: rhs,
            })),
            self.source(location),
        ))
    }

    fn parse_math_rhs(&mut self, operator_precedence: usize) -> Result<Expression, ParseError> {
        // Skip over operator token
        self.input.advance();

        let rhs = self.parse_expression_primary()?;
        let next_operator = self.input.peek();
        let next_precedence = next_operator.kind.precedence();

        if !((next_precedence + is_right_associative(next_operator) as usize) < operator_precedence)
        {
            self.parse_operator_expression(operator_precedence + 1, rhs)
        } else {
            Ok(rhs)
        }
    }

    fn parse_declare_assign(
        &mut self,
        variable_name: String,
        source: Source,
    ) -> Result<Expression, ParseError> {
        // variable_name := value
        //               ^

        self.parse_token(
            TokenKind::DeclareAssign,
            Some("for variable declaration assignment"),
        )?;
        self.ignore_newlines();

        let value = self.parse_expression()?;

        Ok(Expression::new(
            ExpressionKind::DeclareAssign(DeclareAssign {
                name: variable_name,
                value: Box::new(value),
            }),
            source,
        ))
    }

    fn parse_type(
        &mut self,
        prefix: Option<impl ToString>,
        for_reason: Option<impl ToString>,
    ) -> Result<Type, ParseError> {
        match self.input.advance() {
            Token {
                kind: TokenKind::Identifier(identifier),
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
                        if self.input.peek_is(TokenKind::Member) {
                            self.parse_token(TokenKind::Member, None::<&str>)?;
                            let inner = self.parse_type(None::<&str>, None::<&str>)?;
                            Ok(Type::Pointer(Box::new(inner)))
                        } else {
                            Ok(Type::Pointer(Box::new(Type::Void)))
                        }
                    }
                    _ => Err(ParseError {
                        filename: Some(self.input.filename().to_string()),
                        location: Some(location),
                        kind: ParseErrorKind::UndeclaredType { name: identifier },
                    }),
                }
            }
            unexpected => Err(ParseError {
                filename: Some(self.input.filename().to_string()),
                location: Some(unexpected.location),
                kind: ParseErrorKind::ExpectedType {
                    prefix: prefix.map(|prefix| prefix.to_string()),
                    for_reason: for_reason.map(|for_reason| for_reason.to_string()),
                    got: unexpected.to_string(),
                },
            }),
        }
    }

    fn source(&self, location: Location) -> Source {
        Source::new(self.input.key(), location)
    }
}

pub fn parse(
    tokens: impl Iterator<Item = Token>,
    source_file_cache: &SourceFileCache,
    key: SourceFileCacheKey,
) -> Result<Ast, ParseError> {
    Parser::new(Input::new(tokens, source_file_cache, key)).parse()
}

fn is_terminating_token(kind: &TokenKind) -> bool {
    match kind {
        TokenKind::Comma => true,
        TokenKind::CloseParen => true,
        TokenKind::CloseBracket => true,
        TokenKind::CloseCurly => true,
        _ => false,
    }
}

fn is_right_associative(kind: &TokenKind) -> bool {
    match kind {
        TokenKind::DeclareAssign => true,
        _ => false,
    }
}
