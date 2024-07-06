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
        self, Alias, ArrayAccess, Assignment, Ast, BasicBinaryOperation, BasicBinaryOperator,
        BinaryOperator, Block, Call, Conditional, ConformBehavior, Declaration, DeclareAssign,
        Define, Enum, EnumMemberLiteral, Expr, ExprKind, Field, FieldInitializer, File,
        FileIdentifier, FillBehavior, FixedArray, Function, Global, Integer, NamedAlias,
        NamedDefine, NamedEnum, Parameter, Parameters, ShortCircuitingBinaryOperation,
        ShortCircuitingBinaryOperator, Source, Stmt, StmtKind, Structure, Type, TypeKind,
        UnaryOperation, UnaryOperator, While,
    },
    line_column::Location,
    source_file_cache::{SourceFileCache, SourceFileCacheKey},
    token::{StringLiteral, StringModifier, Token, TokenKind},
    try_insert_index_map::try_insert_into_index_map,
};
use ast::FloatSize;
use indexmap::IndexMap;
use itertools::Itertools;
use lazy_format::lazy_format;
use num_bigint::BigInt;
use num_traits::Zero;
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
            TokenKind::AliasKeyword => {
                let NamedAlias { name, alias } = self.parse_alias(annotations)?;
                let source = alias.source;

                try_insert_into_index_map(&mut ast_file.aliases, name, alias, |name| {
                    ParseErrorKind::TypeAliasHasMultipleDefinitions { name }.at(source)
                })?;
            }
            TokenKind::EnumKeyword => {
                let NamedEnum {
                    name,
                    enum_definition,
                } = self.parse_enum(annotations)?;
                let source = enum_definition.source;

                try_insert_into_index_map(&mut ast_file.enums, name, enum_definition, |name| {
                    ParseErrorKind::EnumHasMultipleDefinitions { name }.at(source)
                })?;
            }
            TokenKind::DefineKeyword => {
                let NamedDefine { name, define } = self.parse_define(annotations)?;
                let source = define.source;

                try_insert_into_index_map(&mut ast_file.defines, name, define, |name| {
                    ParseErrorKind::DefineHasMultipleDefinitions { name }.at(source)
                })?;
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
            "pod" => Ok(Annotation::new(AnnotationKind::Pod, location)),
            "abide_abi" => Ok(Annotation::new(AnnotationKind::AbideAbi, location)),
            _ => Err(ParseError {
                kind: ParseErrorKind::UnrecognizedAnnotation {
                    name: annotation_name,
                },
                source: self.source(location),
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
        let source = self.source_here();
        self.input.advance();

        let name = self.parse_identifier(Some("for struct name after 'struct' keyword"))?;
        self.ignore_newlines();

        let mut is_packed = false;
        let mut prefer_pod = false;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Packed => is_packed = true,
                AnnotationKind::Pod => prefer_pod = true,
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

            let source = self.source_here();
            let field_name = self.parse_identifier(Some("for field name"))?;

            self.ignore_newlines();
            let field_type = self.parse_type(None::<&str>, Some("for field type"))?;
            self.ignore_newlines();

            fields.insert(
                field_name,
                Field {
                    ast_type: field_type,
                    privacy: Default::default(),
                    source,
                },
            );
        }

        self.parse_token(TokenKind::CloseParen, Some("to end struct fields"))?;

        Ok(Structure {
            name,
            fields,
            is_packed,
            prefer_pod,
            source,
        })
    }

    fn parse_alias(&mut self, annotations: Vec<Annotation>) -> Result<NamedAlias, ParseError> {
        let source = self.source_here();
        self.input.advance();

        let name = self.parse_identifier(Some("for alias name after 'alias' keyword"))?;
        self.ignore_newlines();

        for annotation in annotations {
            match annotation.kind {
                _ => {
                    return Err(self.unexpected_annotation(
                        annotation.kind.to_string(),
                        annotation.location,
                        Some("for alias"),
                    ))
                }
            }
        }

        self.parse_token(TokenKind::Assign, Some("after alias name"))?;

        let ast_type = self.parse_type(None::<&str>, Some("for alias"))?;

        Ok(NamedAlias {
            name,
            alias: Alias {
                value: ast_type,
                source,
            },
        })
    }

    fn parse_enum(&mut self, annotations: Vec<Annotation>) -> Result<NamedEnum, ParseError> {
        let source = self.source_here();
        self.input.advance();

        let name = self.parse_identifier(Some("for enum name after 'enum' keyword"))?;
        self.ignore_newlines();

        for annotation in annotations {
            match annotation.kind {
                _ => {
                    return Err(self.unexpected_annotation(
                        annotation.kind.to_string(),
                        annotation.location,
                        Some("for enum"),
                    ))
                }
            }
        }

        let mut members = IndexMap::new();

        self.parse_token(TokenKind::OpenParen, Some("after enum name"))?;
        let mut next_value = BigInt::zero();

        while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
            let member_name = self.parse_identifier(Some("for enum member"))?;

            let value = next_value.clone();
            next_value += 1;

            members.insert(
                member_name,
                ast::EnumMember {
                    value,
                    explicit_value: false,
                },
            );

            if !self.input.eat(TokenKind::Comma) && !self.input.peek_is(TokenKind::CloseParen) {
                let got = self.input.advance();
                return Err(self.expected_token(TokenKind::Comma, Some("after enum member"), got));
            }
        }

        self.parse_token(TokenKind::CloseParen, Some("to close enum body"))?;

        Ok(NamedEnum {
            name,
            enum_definition: Enum {
                backing_type: None,
                members,
                source,
            },
        })
    }

    fn parse_define(&mut self, annotations: Vec<Annotation>) -> Result<NamedDefine, ParseError> {
        let source = self.source_here();
        self.input.advance();

        let name = self.parse_identifier(Some("for define name after 'define' keyword"))?;
        self.ignore_newlines();

        self.parse_token(TokenKind::Assign, Some("after name of define"))?;

        for annotation in annotations {
            match annotation.kind {
                _ => {
                    return Err(self.unexpected_annotation(
                        annotation.kind.to_string(),
                        annotation.location,
                        Some("for define"),
                    ))
                }
            }
        }

        let value = self.parse_expr()?;

        Ok(NamedDefine {
            name,
            define: Define { value, source },
        })
    }

    fn parse_function(&mut self, annotations: Vec<Annotation>) -> Result<Function, ParseError> {
        // func functionName {
        //   ^

        let mut is_foreign = false;
        let mut abide_abi = false;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Foreign => is_foreign = true,
                AnnotationKind::AbideAbi => abide_abi = true,
                _ => {
                    return Err(self.unexpected_annotation(
                        annotation.kind.to_string(),
                        annotation.location,
                        Some("for function"),
                    ))
                }
            }
        }

        let location = self.input.advance().location;
        let source = self.source(location);

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
            source,
            abide_abi,
            tag: None,
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
                if self.input.peek_nth(1).kind.could_start_type() {
                    self.parse_declaration()
                } else {
                    let left = self.parse_expr()?;

                    if self.input.peek().is_assignment_like() {
                        self.parse_assignment(left)
                    } else {
                        Ok(Stmt::new(StmtKind::Expr(left), self.source(location)))
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

        if self.input.peek().is_assignment_like() {
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
                StmtKind::Declaration(Box::new(Declaration {
                    name,
                    ast_type,
                    value,
                })),
                self.source(location),
            ))
        }
    }

    fn parse_assignment(&mut self, destination: Expr) -> Result<Stmt, ParseError> {
        let location = self.input.peek().location;

        let operator = match self.input.advance().kind {
            TokenKind::Assign => None,
            TokenKind::AddAssign => Some(BasicBinaryOperator::Add),
            TokenKind::SubtractAssign => Some(BasicBinaryOperator::Subtract),
            TokenKind::MultiplyAssign => Some(BasicBinaryOperator::Multiply),
            TokenKind::DivideAssign => Some(BasicBinaryOperator::Divide),
            TokenKind::ModulusAssign => Some(BasicBinaryOperator::Modulus),
            TokenKind::AmpersandAssign => Some(BasicBinaryOperator::BitwiseAnd),
            TokenKind::PipeAssign => Some(BasicBinaryOperator::BitwiseOr),
            TokenKind::CaretAssign => Some(BasicBinaryOperator::BitwiseXor),
            TokenKind::LeftShiftAssign => Some(BasicBinaryOperator::LeftShift),
            TokenKind::RightShiftAssign => Some(BasicBinaryOperator::RightShift),
            TokenKind::LogicalLeftShiftAssign => Some(BasicBinaryOperator::LogicalLeftShift),
            TokenKind::LogicalRightShiftAssign => Some(BasicBinaryOperator::LogicalRightShift),
            got => {
                return Err(ParseError {
                    source: self.source(location),
                    kind: ParseErrorKind::Expected {
                        expected: "(an assignment operator)".to_string(),
                        for_reason: Some("for assignment".to_string()),
                        got: got.to_string(),
                    },
                })
            }
        };

        let value = self.parse_expr()?;

        Ok(Stmt::new(
            StmtKind::Assignment(Box::new(Assignment {
                destination,
                value,
                operator,
            })),
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

    fn parse_operator_expr(&mut self, precedence: usize, expr: Expr) -> Result<Expr, ParseError> {
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

            let binary_operator: BinaryOperator = match operator.kind {
                TokenKind::Add => BasicBinaryOperator::Add.into(),
                TokenKind::Subtract => BasicBinaryOperator::Subtract.into(),
                TokenKind::Multiply => BasicBinaryOperator::Multiply.into(),
                TokenKind::Divide => BasicBinaryOperator::Divide.into(),
                TokenKind::Modulus => BasicBinaryOperator::Modulus.into(),
                TokenKind::Equals => BasicBinaryOperator::Equals.into(),
                TokenKind::NotEquals => BasicBinaryOperator::NotEquals.into(),
                TokenKind::LessThan => BasicBinaryOperator::LessThan.into(),
                TokenKind::LessThanEq => BasicBinaryOperator::LessThanEq.into(),
                TokenKind::GreaterThan => BasicBinaryOperator::GreaterThan.into(),
                TokenKind::GreaterThanEq => BasicBinaryOperator::GreaterThanEq.into(),
                TokenKind::Ampersand => BasicBinaryOperator::BitwiseAnd.into(),
                TokenKind::Pipe => BasicBinaryOperator::BitwiseOr.into(),
                TokenKind::Caret => BasicBinaryOperator::BitwiseXor.into(),
                TokenKind::LeftShift => BasicBinaryOperator::LeftShift.into(),
                TokenKind::LogicalLeftShift => BasicBinaryOperator::LogicalLeftShift.into(),
                TokenKind::RightShift => BasicBinaryOperator::RightShift.into(),
                TokenKind::LogicalRightShift => BasicBinaryOperator::LogicalRightShift.into(),
                TokenKind::And => ShortCircuitingBinaryOperator::And.into(),
                TokenKind::Or => ShortCircuitingBinaryOperator::Or.into(),
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
                Ok(Expr::new(ExprKind::Boolean(true), self.source(location)))
            }
            TokenKind::FalseKeyword => {
                self.input.advance().kind.unwrap_false_keyword();
                Ok(Expr::new(ExprKind::Boolean(false), self.source(location)))
            }
            TokenKind::Integer(..) => Ok(Expr::new(
                ExprKind::Integer(Integer::Generic(self.input.advance().kind.unwrap_integer())),
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
            TokenKind::StructKeyword | TokenKind::UnionKeyword | TokenKind::EnumKeyword => {
                self.parse_structure_literal()
            }
            TokenKind::Identifier(_) => match self.input.peek_nth(1).kind {
                TokenKind::Namespace => self.parse_enum_member_literal(),
                TokenKind::OpenAngle => self.parse_structure_literal(),
                TokenKind::OpenCurly => {
                    let peek = &self.input.peek_nth(2).kind;

                    if peek.is_extend() || peek.is_colon() {
                        self.parse_structure_literal()
                    } else {
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
                    ExprKind::While(Box::new(While {
                        condition,
                        block: Block::new(stmts),
                    })),
                    self.source(location),
                ))
            }
            unexpected => Err(ParseError {
                kind: match unexpected {
                    TokenKind::Error(message) => ParseErrorKind::Lexical {
                        message: message.into(),
                    },
                    _ => {
                        let unexpected = unexpected.to_string();
                        ParseErrorKind::UnexpectedToken { unexpected }
                    }
                },
                source: self.source(location),
            }),
        }
    }

    fn parse_expr_primary_post(&mut self, mut base: Expr) -> Result<Expr, ParseError> {
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
        // Type { x: VALUE, b: VALUE, c: VALUE, :d, :e, ..SPECIFIER }
        //  ^

        let ast_type = self.parse_type(None::<&str>, Some("for type of struct literal"))?;
        let source = ast_type.source;

        self.parse_token(TokenKind::OpenCurly, Some("to begin struct literal"))?;
        self.ignore_newlines();

        let mut fill_behavior = FillBehavior::Forbid;
        let mut fields = Vec::new();

        while !self.input.peek_is_or_eof(TokenKind::CloseCurly) {
            if self.input.eat(TokenKind::Extend) {
                if self.input.eat(TokenKind::ZeroedKeyword) {
                    fill_behavior = FillBehavior::Zeroed;
                }
            } else {
                let dupe = self.input.eat(TokenKind::Colon);

                let (field_name, field_location) =
                    self.parse_identifier_keep_location(Some("for field name in struct literal"))?;
                self.ignore_newlines();

                let field_value = if dupe {
                    ExprKind::Variable(field_name.clone()).at(self.source(field_location))
                } else {
                    self.parse_token(TokenKind::Colon, Some("after field name in struct literal"))?;
                    self.ignore_newlines();
                    let value = self.parse_expr()?;
                    self.ignore_newlines();
                    value
                };

                fields.push(FieldInitializer {
                    name: Some(field_name),
                    value: field_value,
                });
            }

            self.ignore_newlines();
            if !self.input.peek_is(TokenKind::CloseCurly) {
                self.parse_token(TokenKind::Comma, Some("after field in struct literal"))?;
                self.ignore_newlines();
            }
        }

        self.parse_token(TokenKind::CloseCurly, Some("to end struct literal"))?;
        Ok(Expr::new(
            ExprKind::StructureLiteral(Box::new(ast::StructureLiteral {
                ast_type,
                fields,
                fill_behavior,
                conform_behavior: ConformBehavior::Adept,
            })),
            source,
        ))
    }

    fn parse_enum_member_literal(&mut self) -> Result<Expr, ParseError> {
        // EnumName::EnumVariant
        //    ^

        let source = self.source_here();
        let enum_name = self
            .input
            .eat_identifier()
            .ok_or_else(|| ParseErrorKind::ExpectedEnumName.at(source))?;

        self.parse_token(TokenKind::Namespace, Some("for enum member literal"))?;

        let variant_source = self.source_here();
        let variant_name = self
            .input
            .eat_identifier()
            .ok_or_else(|| ParseErrorKind::ExpectedEnumName.at(variant_source))?;

        Ok(ExprKind::EnumMemberLiteral(Box::new(EnumMemberLiteral {
            enum_name,
            variant_name,
            source,
        }))
        .at(source))
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
            ExprKind::Call(Box::new(Call {
                function_name,
                arguments,
                expected_to_return: None,
            })),
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
            match operator {
                BinaryOperator::Basic(basic_operator) => {
                    ExprKind::BasicBinaryOperation(Box::new(BasicBinaryOperation {
                        operator: basic_operator,
                        left: lhs,
                        right: rhs,
                    }))
                }
                BinaryOperator::ShortCircuiting(short_circuiting_operator) => {
                    ExprKind::ShortCircuitingBinaryOperation(Box::new(
                        ShortCircuitingBinaryOperation {
                            operator: short_circuiting_operator,
                            left: lhs,
                            right: rhs,
                        },
                    ))
                }
            },
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
            ExprKind::DeclareAssign(Box::new(DeclareAssign {
                name: variable_name,
                value,
            })),
            source,
        ))
    }

    /// Parses closing '>' brackets of type parameters.
    /// This function may partially consume tokens, so be
    /// aware that any previously peeked tokens may no longer be in
    /// the same lookahead position after calling this function.
    fn parse_type_parameters_close(&mut self) -> Result<(), ParseError> {
        let closer = self.input.advance();

        /// Sub-function for properly handling trailing `=` signs
        /// resulting from partially consuming '>'-like tokens.
        fn merge_trailing_equals<I: Iterator<Item = Token>>(
            parser: &mut Parser<I>,
            closer: &Token,
            column_offset: u32,
        ) {
            if parser.input.eat(TokenKind::Assign) {
                parser
                    .input
                    .unadvance(TokenKind::Equals.at(closer.location.shift_column(column_offset)));
            } else {
                parser
                    .input
                    .unadvance(TokenKind::Assign.at(closer.location.shift_column(column_offset)));
            }
        }

        match &closer.kind {
            TokenKind::GreaterThan => Ok(()),
            TokenKind::RightShift => {
                self.input
                    .unadvance(TokenKind::GreaterThan.at(closer.location.shift_column(1)));
                Ok(())
            }
            TokenKind::LogicalRightShift => {
                self.input
                    .unadvance(TokenKind::RightShift.at(closer.location.shift_column(1)));
                Ok(())
            }
            TokenKind::RightShiftAssign => {
                merge_trailing_equals(self, &closer, 2);

                self.input
                    .unadvance(TokenKind::GreaterThan.at(closer.location.shift_column(1)));
                Ok(())
            }
            TokenKind::LogicalRightShiftAssign => {
                merge_trailing_equals(self, &closer, 3);

                self.input
                    .unadvance(TokenKind::RightShift.at(closer.location.shift_column(1)));
                Ok(())
            }
            TokenKind::GreaterThanEq => {
                merge_trailing_equals(self, &closer, 1);
                Ok(())
            }
            _ => Err(self.unexpected_token(&closer)),
        }
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
                        if self.input.eat(TokenKind::OpenAngle) {
                            let inner = self.parse_type(None::<&str>, None::<&str>)?;
                            self.parse_type_parameters_close()?;
                            Ok(TypeKind::Pointer(Box::new(inner)))
                        } else {
                            Ok(TypeKind::Pointer(Box::new(Type::new(
                                TypeKind::Void,
                                source,
                            ))))
                        }
                    }
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
                    "pod" => {
                        self.parse_token(
                            TokenKind::OpenAngle,
                            Some("to specify inner type of 'pod'"),
                        )?;
                        let inner = self.parse_type(None::<&str>, None::<&str>)?;
                        self.parse_type_parameters_close()?;
                        Ok(TypeKind::PlainOldData(Box::new(inner)))
                    }
                    identifier => Ok(TypeKind::Named(identifier.into())),
                }?;

                Ok(Type::new(type_kind, source))
            }
            Token {
                kind: TokenKind::StructKeyword,
                ..
            } => Ok(TypeKind::Named(format!(
                "struct<{}>",
                self.parse_name_type_parameters(source)?
            ))
            .at(source)),
            Token {
                kind: TokenKind::UnionKeyword,
                ..
            } => Ok(TypeKind::Named(format!(
                "union<{}>",
                self.parse_name_type_parameters(source)?
            ))
            .at(source)),
            Token {
                kind: TokenKind::EnumKeyword,
                ..
            } => Ok(TypeKind::Named(format!(
                "enum<{}>",
                self.parse_name_type_parameters(source)?
            ))
            .at(source)),
            unexpected => Err(ParseError {
                kind: ParseErrorKind::ExpectedType {
                    prefix: prefix.map(|prefix| prefix.to_string()),
                    for_reason: for_reason.map(|for_reason| for_reason.to_string()),
                    got: unexpected.to_string(),
                },
                source: self.source(unexpected.location),
            }),
        }
    }

    fn parse_name_type_parameters(&mut self, source: Source) -> Result<String, ParseError> {
        if !self.input.eat(TokenKind::OpenAngle) {
            return Err(ParseError {
                kind: ParseErrorKind::ExpectedTypeParameters,
                source,
            });
        }

        let name = self.input.eat_identifier().ok_or_else(|| ParseError {
            kind: ParseErrorKind::ExpectedTypeName,
            source: self.source_here(),
        })?;

        self.parse_type_parameters_close()?;
        Ok(name)
    }

    fn source(&self, location: Location) -> Source {
        Source::new(self.input.key(), location)
    }

    fn source_here(&mut self) -> Source {
        Source::new(self.input.key(), self.input.peek().location)
    }
}

pub fn parse(
    tokens: impl Iterator<Item = Token>,
    source_file_cache: &SourceFileCache,
    key: SourceFileCacheKey,
) -> Result<Ast, ParseError> {
    Parser::new(Input::new(tokens, source_file_cache, key)).parse()
}

pub fn parse_into(
    tokens: impl Iterator<Item = Token>,
    source_file_cache: &SourceFileCache,
    key: SourceFileCacheKey,
    ast: &mut Ast,
    filename: String,
) -> Result<(), ParseError> {
    Parser::new(Input::new(tokens, source_file_cache, key)).parse_into(ast, filename)
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
