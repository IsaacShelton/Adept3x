mod annotation;
mod error;
mod input;
mod make_error;

use self::{
    annotation::{Annotation, AnnotationKind},
    error::{ParseError, ParseErrorKind},
    input::Input,
};
use crate::{
    ast::{
        self, ArrayAccess, Assignment, Ast, BinaryOperation, Block, Call, Conditional, Declaration,
        DeclareAssign, Expr, ExprKind, Field, File, FileIdentifier, Function, Global,
        Parameter, Parameters, Source, Stmt, StmtKind, Structure, Type, TypeKind,
        UnaryOperation, UnaryOperator, While,
    },
    line_column::Location,
    source_file_cache::{SourceFileCache, SourceFileCacheKey},
    token::{StringLiteral, StringModifier, Token, TokenKind},
};
use ast::{BinaryOperator, FloatSize};
use indexmap::IndexMap;
use itertools::Itertools;
use lazy_format::lazy_format;
use std::{borrow::Borrow, ffi::CString};

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
            self.parse_top_level(ast_file, vec![])?;
        }

        Ok(())
    }

    fn parse_top_level(
        &mut self,
        ast_file: &mut File,
        parent_annotations: Vec<Annotation>,
    ) -> Result<(), ParseError> {
        let mut annotations = parent_annotations;

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
            TokenKind::OpenCurly => {
                self.input.advance().kind.unwrap_open_curly();
                self.ignore_newlines();

                while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
                    self.parse_top_level(ast_file, annotations.clone())?;
                    self.ignore_newlines();
                }

                self.parse_token(TokenKind::CloseCurly, Some("to close annotation group"))?;
            }
            TokenKind::FuncKeyword => {
                ast_file.functions.push(self.parse_function(annotations)?);
            }
            TokenKind::Identifier(_) => {
                ast_file.globals.push(self.parse_global(annotations)?);
            }
            TokenKind::StructKeyword => {
                ast_file.structures.push(self.parse_structure(annotations)?)
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
            "packed" => Ok(Annotation::new(AnnotationKind::Packed, location)),
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
                _ => {
                    return Err(self.unexpected_annotation(
                        annotation.kind.to_string(),
                        annotation.location,
                        Some("for global variable"),
                    ))
                }
            }
        }

        let (name, location) =
            self.parse_identifier_keep_location(Some("for name of global variable"))?;
        let ast_type = self.parse_type(None::<&str>, Some("for type of global variable"))?;

        Ok(Global {
            name,
            ast_type,
            source: self.source(location),
            is_foreign,
            is_thread_local,
        })
    }

    fn parse_structure(&mut self, annotations: Vec<Annotation>) -> Result<Structure, ParseError> {
        self.input.advance();

        let name = self.parse_identifier(Some("for struct name after 'struct' keyword"))?;
        self.ignore_newlines();

        let mut is_packed = false;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Packed => is_packed = true,
                _ => {
                    return Err(self.unexpected_annotation(
                        annotation.kind.to_string(),
                        annotation.location,
                        Some("for structure"),
                    ))
                }
            }
        }

        let mut fields = IndexMap::new();

        self.ignore_newlines();
        self.parse_token(TokenKind::OpenParen, Some("to begin struct fields"))?;

        while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
            if fields.len() != 0 {
                self.parse_token(TokenKind::Comma, Some("to separate struct fields"))?;
                self.ignore_newlines();
            }

            let field_name = self.parse_identifier(Some("for field name"))?;
            self.ignore_newlines();
            let field_type = self.parse_type(None::<&str>, Some("for field type"))?;
            self.ignore_newlines();

            fields.insert(
                field_name,
                Field {
                    ast_type: field_type,
                    privacy: Default::default(),
                },
            );
        }

        self.parse_token(TokenKind::CloseParen, Some("to end struct fields"))?;

        Ok(Structure {
            name,
            fields,
            is_packed,
        })
    }

    fn parse_function(&mut self, annotations: Vec<Annotation>) -> Result<Function, ParseError> {
        // func functionName {
        //   ^

        let mut is_foreign = false;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Foreign => is_foreign = true,
                _ => {
                    return Err(self.unexpected_annotation(
                        annotation.kind.to_string(),
                        annotation.location,
                        Some("for function"),
                    ))
                }
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
            let location = self.input.peek().location;
            ast::Type::new(ast::TypeKind::Void, self.source(location))
        } else {
            self.parse_type(Some("return "), Some("for function"))?
        };

        let stmts = (!is_foreign)
            .then(|| self.parse_block("function"))
            .transpose()?
            .unwrap_or_default();

        Ok(Function {
            name,
            parameters,
            return_type,
            stmts,
            is_foreign,
        })
    }

    fn parse_block(&mut self, to_begin_what_block: &str) -> Result<Vec<Stmt>, ParseError> {
        self.ignore_newlines();

        self.parse_token(
            TokenKind::OpenCurly,
            Some(lazy_format!("to begin {} block", to_begin_what_block)),
        )?;

        let mut stmts = Vec::new();
        self.ignore_newlines();

        while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
            stmts.push(self.parse_stmt()?);
            self.ignore_newlines();
        }

        self.ignore_newlines();
        self.parse_token(
            TokenKind::CloseCurly,
            Some(lazy_format!("to close {} block", to_begin_what_block)),
        )?;

        Ok(stmts)
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

    fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        let location = self.input.peek().location;

        match self.input.peek().kind {
            TokenKind::Identifier(_) => {
                if let TokenKind::Identifier(..) = self.input.peek_nth(1).kind {
                    self.parse_declaration()
                } else {
                    let left = self.parse_expr()?;

                    if self.input.peek_is(TokenKind::Assign) {
                        self.parse_assignment(left)
                    } else {
                        Ok(Stmt::new(
                            StmtKind::Expr(left),
                            self.source(location),
                        ))
                    }
                }
            }
            TokenKind::ReturnKeyword => self.parse_return(),
            TokenKind::EndOfFile => Err(self.unexpected_token_is_next()),
            _ => Ok(Stmt::new(
                StmtKind::Expr(self.parse_expr()?),
                self.source(location),
            )),
        }
    }

    fn parse_declaration(&mut self) -> Result<Stmt, ParseError> {
        let (name, location) = self.parse_identifier_keep_location(Some("for variable name"))?;

        if self.input.peek_is(TokenKind::Assign) {
            let variable = Expr::new(ExprKind::Variable(name), self.source(location));
            self.parse_assignment(variable)
        } else {
            let ast_type = self.parse_type(None::<&str>, Some("for variable type"))?;

            let value = if self.input.peek_is(TokenKind::Assign) {
                self.input.advance();
                Some(self.parse_expr()?)
            } else {
                None
            };

            self.ignore_newlines();

            Ok(Stmt::new(
                StmtKind::Declaration(Declaration {
                    name,
                    ast_type,
                    value,
                }),
                self.source(location),
            ))
        }
    }

    fn parse_assignment(&mut self, destination: Expr) -> Result<Stmt, ParseError> {
        let location = self.parse_token(TokenKind::Assign, Some("for assignment"))?;
        let value = self.parse_expr()?;

        Ok(Stmt::new(
            StmtKind::Assignment(Assignment { destination, value }),
            self.source(location),
        ))
    }

    fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        // return VALUE
        //          ^

        let location = self.parse_token(TokenKind::ReturnKeyword, Some("for return statement"))?;

        Ok(Stmt::new(
            StmtKind::Return(if self.input.peek_is(TokenKind::Newline) {
                None
            } else {
                Some(self.parse_expr()?)
            }),
            self.source(location),
        ))
    }

    fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let primary = self.parse_expr_primary()?;
        self.parse_operator_expr(0, primary)
    }

    fn parse_operator_expr(
        &mut self,
        precedence: usize,
        expr: Expr,
    ) -> Result<Expr, ParseError> {
        let mut lhs = expr;

        loop {
            let operator = self.input.peek();
            let location = operator.location;
            let next_precedence = operator.kind.precedence();

            if is_terminating_token(&operator.kind)
                || (next_precedence + is_right_associative(operator) as usize) < precedence
            {
                return Ok(lhs);
            }

            let binary_operator = match operator.kind {
                TokenKind::Add => BinaryOperator::Add,
                TokenKind::Subtract => BinaryOperator::Subtract,
                TokenKind::Multiply => BinaryOperator::Multiply,
                TokenKind::Divide => BinaryOperator::Divide,
                TokenKind::Modulus => BinaryOperator::Modulus,
                TokenKind::Equals => BinaryOperator::Equals,
                TokenKind::NotEquals => BinaryOperator::NotEquals,
                TokenKind::LessThan => BinaryOperator::LessThan,
                TokenKind::LessThanEq => BinaryOperator::LessThanEq,
                TokenKind::GreaterThan => BinaryOperator::GreaterThan,
                TokenKind::GreaterThanEq => BinaryOperator::GreaterThanEq,
                TokenKind::Ampersand => BinaryOperator::BitwiseAnd,
                TokenKind::Pipe => BinaryOperator::BitwiseOr,
                TokenKind::Caret => BinaryOperator::BitwiseXor,
                TokenKind::LeftShift => BinaryOperator::LeftShift,
                TokenKind::LogicalLeftShift => BinaryOperator::LogicalLeftShift,
                TokenKind::RightShift => BinaryOperator::RightShift,
                TokenKind::LogicalRightShift => BinaryOperator::LogicalRightShift,
                _ => return Ok(lhs),
            };

            lhs = self.parse_math(lhs, binary_operator, next_precedence, location)?;
        }
    }

    fn parse_expr_primary(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_expr_primary_base()?;
        self.parse_expr_primary_post(expr)
    }

    fn parse_expr_primary_base(&mut self) -> Result<Expr, ParseError> {
        let Token { kind, location } = self.input.peek();
        let location = *location;

        match kind {
            TokenKind::TrueKeyword => {
                self.input.advance().kind.unwrap_true_keyword();
                Ok(Expr::new(
                    ExprKind::Boolean(true),
                    self.source(location),
                ))
            }
            TokenKind::FalseKeyword => {
                self.input.advance().kind.unwrap_false_keyword();
                Ok(Expr::new(
                    ExprKind::Boolean(false),
                    self.source(location),
                ))
            }
            TokenKind::Integer(..) => Ok(Expr::new(
                ExprKind::Integer(self.input.advance().kind.unwrap_integer()),
                self.source(location),
            )),
            TokenKind::Float(..) => Ok(Expr::new(
                ExprKind::Float(self.input.advance().kind.unwrap_float()),
                self.source(location),
            )),
            TokenKind::String(StringLiteral {
                modifier: StringModifier::NullTerminated,
                ..
            }) => Ok(Expr::new(
                ExprKind::NullTerminatedString(
                    CString::new(self.input.advance().kind.unwrap_string().value)
                        .expect("valid null-terminated string"),
                ),
                self.source(location),
            )),
            TokenKind::String(StringLiteral {
                modifier: StringModifier::Normal,
                ..
            }) => Ok(Expr::new(
                ExprKind::String(self.input.advance().kind.unwrap_string().value),
                self.source(location),
            )),
            TokenKind::OpenParen => {
                self.input.advance().kind.unwrap_open_paren();
                let inner = self.parse_expr()?;
                self.parse_token(TokenKind::CloseParen, Some("to close nested expression"))?;
                Ok(inner)
            }
            TokenKind::Identifier(_) => match self.input.peek_nth(1).kind {
                TokenKind::OpenAngle => self.parse_structure_literal(),
                TokenKind::OpenCurly => {
                    let next_three = self
                        .input
                        .peek_n(5)
                        .iter()
                        .skip(2)
                        .map(|token| &token.kind)
                        .collect_vec();

                    match &next_three[..] {
                        [TokenKind::Identifier(_), TokenKind::Colon, ..]
                        | [TokenKind::Newline, TokenKind::Identifier(_), TokenKind::Colon, ..] => {
                            self.parse_structure_literal()
                        }
                        _ => Ok(Expr::new(
                            ExprKind::Variable(self.input.advance().kind.unwrap_identifier()),
                            self.source(location),
                        )),
                    }
                }
                TokenKind::OpenParen => self.parse_call(),
                TokenKind::DeclareAssign => self.parse_declare_assign(),
                _ => Ok(Expr::new(
                    ExprKind::Variable(self.input.advance().kind.unwrap_identifier()),
                    self.source(location),
                )),
            },
            TokenKind::Not | TokenKind::BitComplement | TokenKind::Subtract => {
                let operator = match kind {
                    TokenKind::Not => UnaryOperator::Not,
                    TokenKind::BitComplement => UnaryOperator::BitComplement,
                    TokenKind::Subtract => UnaryOperator::Negate,
                    _ => unreachable!(),
                };

                let location = self.input.advance().location;
                let inner = self.parse_expr()?;

                Ok(Expr::new(
                    ExprKind::UnaryOperation(Box::new(UnaryOperation { operator, inner })),
                    self.source(location),
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

                Ok(Expr::new(
                    ExprKind::Conditional(conditional),
                    self.source(location),
                ))
            }
            TokenKind::WhileKeyword => {
                self.input.advance().kind.unwrap_while_keyword();
                self.ignore_newlines();

                let condition = self.parse_expr()?;
                let stmts = self.parse_block("'while'")?;

                Ok(Expr::new(
                    ExprKind::While(While {
                        condition: Box::new(condition),
                        block: Block::new(stmts),
                    }),
                    self.source(location),
                ))
            }
            unexpected => {
                let unexpected = unexpected.to_string();
                Err(ParseError {
                    filename: Some(self.input.filename().to_string()),
                    location: Some(location),
                    kind: ParseErrorKind::UnexpectedToken { unexpected },
                })
            }
        }
    }

    fn parse_expr_primary_post(
        &mut self,
        mut base: Expr,
    ) -> Result<Expr, ParseError> {
        loop {
            self.ignore_newlines();

            match self.input.peek().kind {
                TokenKind::Member => base = self.parse_member(base)?,
                TokenKind::OpenBracket => base = self.parse_array_access(base)?,
                _ => break,
            }
        }

        Ok(base)
    }

    fn parse_member(&mut self, subject: Expr) -> Result<Expr, ParseError> {
        // subject.field_name
        //        ^

        let location = self.parse_token(TokenKind::Member, Some("for member expression"))?;
        let field_name = self.parse_identifier(Some("for field name"))?;

        Ok(Expr::new(
            ExprKind::Member(Box::new(subject), field_name),
            self.source(location),
        ))
    }

    fn parse_array_access(&mut self, subject: Expr) -> Result<Expr, ParseError> {
        // subject[index]
        //        ^

        let location = self.parse_token(TokenKind::OpenBracket, Some("for array access"))?;

        self.ignore_newlines();
        let index = self.parse_expr()?;
        self.ignore_newlines();

        self.parse_token(TokenKind::CloseBracket, Some("to close array access"))?;

        Ok(Expr::new(
            ExprKind::ArrayAccess(Box::new(ArrayAccess { subject, index })),
            self.source(location),
        ))
    }

    fn parse_structure_literal(&mut self) -> Result<Expr, ParseError> {
        // Type { x: VALUE, b: VALUE, c: VALUE }
        //  ^

        let ast_type = self.parse_type(None::<&str>, Some("for type of struct literal"))?;
        let source = ast_type.source;

        self.parse_token(TokenKind::OpenCurly, Some("to begin struct literal"))?;
        self.ignore_newlines();

        let mut fields = IndexMap::new();

        while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
            let (field_name, field_location) =
                self.parse_identifier_keep_location(Some("for field name in struct literal"))?;
            self.ignore_newlines();

            self.parse_token(TokenKind::Colon, Some("after field name in struct literal"))?;
            self.ignore_newlines();

            let field_value = self.parse_expr()?;
            self.ignore_newlines();

            if fields.get(&field_name).is_some() {
                return Err(ParseError {
                    filename: Some(self.input.filename().to_string()),
                    location: Some(field_location),
                    kind: ParseErrorKind::FieldSpecifiedMoreThanOnce { field_name },
                });
            }

            fields.insert(field_name, field_value);

            self.ignore_newlines();
            if !self.input.peek_is(TokenKind::CloseCurly) {
                self.parse_token(TokenKind::Comma, Some("after field in struct literal"))?;
                self.ignore_newlines();
            }
        }

        self.parse_token(TokenKind::CloseCurly, Some("to end struct literal"))?;
        Ok(Expr::new(
            ExprKind::StructureLiteral(ast_type, fields),
            source,
        ))
    }

    fn parse_call(&mut self) -> Result<Expr, ParseError> {
        // function_name(arg1, arg2, arg3)
        //              ^

        let (function_name, location) =
            self.parse_identifier_keep_location(Some("for function call"))?;
        let source = self.source(location);
        let mut arguments = vec![];

        self.parse_token(TokenKind::OpenParen, Some("to begin call argument list"))?;
        self.ignore_newlines();

        while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
            if !arguments.is_empty() {
                self.parse_token(TokenKind::Comma, Some("to separate arguments"))?;
                self.ignore_newlines();
            }

            arguments.push(self.parse_expr()?);
            self.ignore_newlines();
        }

        self.parse_token(TokenKind::CloseParen, Some("to end call argument list"))?;

        Ok(Expr::new(
            ExprKind::Call(Call {
                function_name,
                arguments,
            }),
            source,
        ))
    }

    fn parse_math(
        &mut self,
        lhs: Expr,
        operator: BinaryOperator,
        operator_precedence: usize,
        location: Location,
    ) -> Result<Expr, ParseError> {
        let rhs = self.parse_math_rhs(operator_precedence)?;

        Ok(Expr::new(
            ExprKind::BinaryOperation(Box::new(BinaryOperation {
                operator,
                left: lhs,
                right: rhs,
            })),
            self.source(location),
        ))
    }

    fn parse_math_rhs(&mut self, operator_precedence: usize) -> Result<Expr, ParseError> {
        // Skip over operator token
        self.input.advance();

        let rhs = self.parse_expr_primary()?;
        let next_operator = self.input.peek();
        let next_precedence = next_operator.kind.precedence();

        if !((next_precedence + is_right_associative(next_operator) as usize) < operator_precedence)
        {
            self.parse_operator_expr(operator_precedence + 1, rhs)
        } else {
            Ok(rhs)
        }
    }

    fn parse_declare_assign(&mut self) -> Result<Expr, ParseError> {
        // variable_name := value
        //               ^

        let (variable_name, location) =
            self.parse_identifier_keep_location(Some("for function call"))?;
        let source = self.source(location);

        self.parse_token(
            TokenKind::DeclareAssign,
            Some("for variable declaration assignment"),
        )?;
        self.ignore_newlines();

        let value = self.parse_expr()?;

        Ok(Expr::new(
            ExprKind::DeclareAssign(DeclareAssign {
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
        let location = self.input.peek().location;
        let source = self.source(location);

        match self.input.advance() {
            Token {
                kind: TokenKind::Identifier(identifier),
                ..
            } => {
                use ast::{IntegerBits::*, IntegerSign::*};

                let type_kind = match identifier.as_str() {
                    "bool" => Ok(TypeKind::Boolean),
                    "int" => Ok(TypeKind::Integer {
                        bits: Normal,
                        sign: Signed,
                    }),
                    "uint" => Ok(TypeKind::Integer {
                        bits: Normal,
                        sign: Unsigned,
                    }),
                    "i8" => Ok(TypeKind::Integer {
                        bits: Bits8,
                        sign: Signed,
                    }),
                    "u8" => Ok(TypeKind::Integer {
                        bits: Bits8,
                        sign: Unsigned,
                    }),
                    "i16" => Ok(TypeKind::Integer {
                        bits: Bits16,
                        sign: Signed,
                    }),
                    "u16" => Ok(TypeKind::Integer {
                        bits: Bits16,
                        sign: Unsigned,
                    }),
                    "i32" => Ok(TypeKind::Integer {
                        bits: Bits32,
                        sign: Signed,
                    }),
                    "u32" => Ok(TypeKind::Integer {
                        bits: Bits32,
                        sign: Unsigned,
                    }),
                    "i64" => Ok(TypeKind::Integer {
                        bits: Bits64,
                        sign: Signed,
                    }),
                    "u64" => Ok(TypeKind::Integer {
                        bits: Bits64,
                        sign: Unsigned,
                    }),
                    "float" => Ok(TypeKind::Float(FloatSize::Normal)),
                    "f32" => Ok(TypeKind::Float(FloatSize::Bits32)),
                    "f64" => Ok(TypeKind::Float(FloatSize::Bits64)),
                    "void" => Ok(TypeKind::Void),
                    "ptr" => {
                        if self.input.peek_is(TokenKind::OpenAngle) {
                            self.input.advance();
                            let inner = self.parse_type(None::<&str>, None::<&str>)?;
                            self.parse_token(
                                TokenKind::GreaterThan,
                                Some("to close type parameters"),
                            )?;
                            Ok(TypeKind::Pointer(Box::new(inner)))
                        } else {
                            Ok(TypeKind::Pointer(Box::new(Type::new(
                                TypeKind::Void,
                                source,
                            ))))
                        }
                    }
                    "pod" => {
                        self.parse_token(TokenKind::OpenAngle, Some("to specify inner type of 'pod'"))?;
                        let inner = self.parse_type(None::<&str>, None::<&str>)?;
                        self.parse_token(
                            TokenKind::GreaterThan,
                            Some("to close type parameters"),
                        )?;
                        Ok(TypeKind::PlainOldData(Box::new(inner)))
                    }
                    "unsync" => {
                        self.parse_token(TokenKind::OpenAngle, Some("to specify inner type of 'unsync'"))?;
                        let inner = self.parse_type(None::<&str>, None::<&str>)?;
                        self.parse_token(
                            TokenKind::GreaterThan,
                            Some("to close type parameters"),
                        )?;
                        Ok(TypeKind::Unsync(Box::new(inner)))
                    }
                    identifier => Ok(TypeKind::Named(identifier.into())),
                }?;

                Ok(Type::new(type_kind, source))
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
