mod annotation;
pub mod error;
mod input;
mod make_error;

pub use self::input::Input;
use self::{
    annotation::{Annotation, AnnotationKind},
    error::{ParseError, ParseErrorKind},
};
use crate::{
    ast::{
        self, ArrayAccess, Assignment, AstFile, BasicBinaryOperation, BasicBinaryOperator,
        BinaryOperator, Block, Call, Conditional, ConformBehavior, Declaration, DeclareAssign,
        Enum, EnumMemberLiteral, Expr, ExprKind, Field, FieldInitializer, FillBehavior, FixedArray,
        Function, GlobalVar, HelperExpr, Integer, Named, Parameter, Parameters,
        ShortCircuitingBinaryOperation, ShortCircuitingBinaryOperator, Stmt, StmtKind, Structure,
        Type, TypeAlias, TypeKind, UnaryOperation, UnaryOperator, While,
    },
    index_map_ext::IndexMapExt,
    inflow::Inflow,
    source_files::{Source, SourceFileKey, SourceFiles},
    token::{StringLiteral, StringModifier, Token, TokenKind},
};
use ast::FloatSize;
use indexmap::IndexMap;
use lazy_format::lazy_format;
use num_bigint::BigInt;
use num_traits::Zero;
use std::{borrow::Borrow, ffi::CString, mem::MaybeUninit};

pub fn parse(
    tokens: impl Inflow<Token>,
    source_files: &SourceFiles,
    key: SourceFileKey,
) -> Result<AstFile, ParseError> {
    Parser::new(Input::new(tokens, source_files, key)).parse()
}

pub struct Parser<'a, I: Inflow<Token>> {
    pub input: Input<'a, I>,
}

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn new(input: Input<'a, I>) -> Self {
        Self { input }
    }

    pub fn parse(&mut self) -> Result<AstFile, ParseError> {
        let mut ast_file = AstFile::new();

        // Parse into ast file
        while !self.input.peek().is_end_of_file() {
            self.parse_top_level(&mut ast_file, vec![])?;
        }

        Ok(ast_file)
    }

    pub fn parse_top_level(
        &mut self,
        ast_file: &mut AstFile,
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
                ast_file
                    .global_variables
                    .push(self.parse_global_variable(annotations)?);
            }
            TokenKind::StructKeyword => {
                ast_file.structures.push(self.parse_structure(annotations)?)
            }
            TokenKind::AliasKeyword => {
                let Named::<TypeAlias> { name, value: alias } = self.parse_alias(annotations)?;
                let source = alias.source;

                ast_file.type_aliases.try_insert(name, alias, |name| {
                    ParseErrorKind::TypeAliasHasMultipleDefinitions { name }.at(source)
                })?;
            }
            TokenKind::EnumKeyword => {
                let Named::<Enum> {
                    name,
                    value: enum_definition,
                } = self.parse_enum(annotations)?;

                let source = enum_definition.source;

                ast_file.enums.try_insert(name, enum_definition, |name| {
                    ParseErrorKind::EnumHasMultipleDefinitions { name }.at(source)
                })?;
            }
            TokenKind::DefineKeyword => {
                let Named::<HelperExpr> {
                    name,
                    value: named_expr,
                } = self.parse_helper_expr(annotations)?;
                let source = named_expr.source;

                ast_file.helper_exprs.try_insert(name, named_expr, |name| {
                    ParseErrorKind::DefineHasMultipleDefinitions { name }.at(source)
                })?;
            }
            TokenKind::EndOfFile => {
                // End-of-file is only okay if no preceeding annotations
                if !annotations.is_empty() {
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
    ) -> Result<Source, ParseError> {
        let token = self.input.advance();
        let expected_token = expected_token.borrow();

        if token.kind == *expected_token {
            return Ok(token.source);
        }

        Err(ParseError::expected(expected_token, for_reason, token))
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
    ) -> Result<(String, Source), ParseError> {
        let token = self.input.advance();

        if let TokenKind::Identifier(identifier) = &token.kind {
            Ok((identifier.into(), token.source))
        } else {
            Err(ParseError::expected("identifier", for_reason, token))
        }
    }

    fn ignore_newlines(&mut self) {
        while self.input.peek().kind.is_newline() {
            self.input.advance();
        }
    }

    fn parse_annotation(&mut self) -> Result<Annotation, ParseError> {
        // #[annotation_name]
        // ^

        self.parse_token(TokenKind::Hash, Some("to begin annotation"))?;
        self.parse_token(TokenKind::OpenBracket, Some("to begin annotation body"))?;

        let (annotation_name, source) =
            self.parse_identifier_keep_location(Some("for annotation name"))?;

        self.parse_token(TokenKind::CloseBracket, Some("to close annotation body"))?;

        match annotation_name.as_str() {
            "foreign" => Ok(Annotation::new(AnnotationKind::Foreign, source)),
            "thread_local" => Ok(Annotation::new(AnnotationKind::ThreadLocal, source)),
            "packed" => Ok(Annotation::new(AnnotationKind::Packed, source)),
            "pod" => Ok(Annotation::new(AnnotationKind::Pod, source)),
            "abide_abi" => Ok(Annotation::new(AnnotationKind::AbideAbi, source)),
            _ => Err(ParseError {
                kind: ParseErrorKind::UnrecognizedAnnotation {
                    name: annotation_name,
                },
                source,
            }),
        }
    }

    fn parse_global_variable(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<GlobalVar, ParseError> {
        // my_global_name Type
        //      ^

        let mut is_foreign = false;
        let mut is_thread_local = false;

        for annotation in annotations {
            match annotation.kind {
                AnnotationKind::Foreign => is_foreign = true,
                AnnotationKind::ThreadLocal => is_thread_local = true,
                _ => {
                    return Err(self.unexpected_annotation(&annotation, Some("for global variable")))
                }
            }
        }

        let (name, source) =
            self.parse_identifier_keep_location(Some("for name of global variable"))?;

        // Better error message for trying to call functions at global scope
        if self.input.peek_is(TokenKind::OpenParen) {
            return Err(ParseErrorKind::CannotCallFunctionsAtGlobalScope.at(source));
        }

        let ast_type = self.parse_type(None::<&str>, Some("for type of global variable"))?;

        Ok(GlobalVar {
            name,
            ast_type,
            source,
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
                _ => return Err(self.unexpected_annotation(&annotation, Some("for structure"))),
            }
        }

        let mut fields = IndexMap::new();

        self.ignore_newlines();
        self.parse_token(TokenKind::OpenParen, Some("to begin struct fields"))?;

        while !self.input.peek_is_or_eof(TokenKind::CloseParen) {
            if !fields.is_empty() {
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

    fn parse_alias(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<Named<TypeAlias>, ParseError> {
        let source = self.source_here();
        self.input.advance();

        let name = self.parse_identifier(Some("for alias name after 'alias' keyword"))?;
        self.ignore_newlines();

        #[allow(clippy::never_loop, clippy::match_single_binding)]
        for annotation in annotations {
            match annotation.kind {
                _ => return Err(self.unexpected_annotation(&annotation, Some("for alias"))),
            }
        }

        self.parse_token(TokenKind::Assign, Some("after alias name"))?;

        let ast_type = self.parse_type(None::<&str>, Some("for alias"))?;

        Ok(Named::<TypeAlias> {
            name,
            value: TypeAlias {
                value: ast_type,
                source,
            },
        })
    }

    fn parse_enum(&mut self, annotations: Vec<Annotation>) -> Result<Named<Enum>, ParseError> {
        let source = self.source_here();
        self.input.advance();

        let name = self.parse_identifier(Some("for enum name after 'enum' keyword"))?;
        self.ignore_newlines();

        #[allow(clippy::never_loop, clippy::match_single_binding)]
        for annotation in annotations {
            match annotation.kind {
                _ => return Err(self.unexpected_annotation(&annotation, Some("for enum"))),
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
                return Err(ParseError::expected(
                    TokenKind::Comma,
                    Some("after enum member"),
                    got,
                ));
            }
        }

        self.parse_token(TokenKind::CloseParen, Some("to close enum body"))?;

        Ok(Named::<Enum> {
            name,
            value: Enum {
                backing_type: None,
                members,
                source,
            },
        })
    }

    fn parse_helper_expr(
        &mut self,
        annotations: Vec<Annotation>,
    ) -> Result<Named<HelperExpr>, ParseError> {
        let source = self.source_here();
        self.input.advance();

        let name = self.parse_identifier(Some("for define name after 'define' keyword"))?;
        self.ignore_newlines();

        self.parse_token(TokenKind::Assign, Some("after name of define"))?;

        #[allow(clippy::never_loop, clippy::match_single_binding)]
        for annotation in annotations {
            match annotation.kind {
                _ => return Err(self.unexpected_annotation(&annotation, Some("for define"))),
            }
        }

        let value = self.parse_expr()?;

        Ok(Named::<HelperExpr> {
            value: HelperExpr {
                value,
                source,
                is_file_local_only: false,
            },
            name,
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
                _ => return Err(self.unexpected_annotation(&annotation, Some("for function"))),
            }
        }

        // abide_abi is implied for all foreign functions
        if is_foreign {
            abide_abi = true;
        }

        let source = self.input.advance().source;

        let name = self.parse_identifier(Some("after 'func' keyword"))?;
        self.ignore_newlines();

        let parameters = if self.input.peek_is(TokenKind::OpenParen) {
            self.parse_function_parameters()?
        } else {
            Parameters::default()
        };

        self.ignore_newlines();

        let return_type = if self.input.peek_is(TokenKind::OpenCurly) {
            ast::TypeKind::Void.at(self.source_here())
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

    pub fn parse_block(&mut self, to_begin_what_block: &str) -> Result<Vec<Stmt>, ParseError> {
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
        let source = self.source_here();

        match self.input.peek().kind {
            TokenKind::Identifier(_) => {
                if self.input.peek_nth(1).kind.could_start_type() {
                    self.parse_declaration()
                } else {
                    let left = self.parse_expr()?;

                    if self.input.peek().is_assignment_like() {
                        self.parse_assignment(left)
                    } else {
                        Ok(StmtKind::Expr(left).at(source))
                    }
                }
            }
            TokenKind::ReturnKeyword => self.parse_return(),
            TokenKind::EndOfFile => Err(self.unexpected_token_is_next()),
            _ => Ok(Stmt::new(StmtKind::Expr(self.parse_expr()?), source)),
        }
    }

    fn parse_declaration(&mut self) -> Result<Stmt, ParseError> {
        let (name, source) = self.parse_identifier_keep_location(Some("for variable name"))?;

        if self.input.peek().is_assignment_like() {
            let variable = ExprKind::Variable(name).at(source);
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
                source,
            ))
        }
    }

    fn parse_assignment(&mut self, destination: Expr) -> Result<Stmt, ParseError> {
        let source = self.input.peek().source;

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
                return Err(ParseError::expected(
                    "(an assignment operator)",
                    Some("for assignment"),
                    got.at(source),
                ))
            }
        };

        let value = self.parse_expr()?;

        Ok(StmtKind::Assignment(Box::new(Assignment {
            destination,
            value,
            operator,
        }))
        .at(source))
    }

    fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        // return VALUE
        //          ^

        let source = self.parse_token(TokenKind::ReturnKeyword, Some("for return statement"))?;

        Ok(StmtKind::Return(if self.input.peek_is(TokenKind::Newline) {
            None
        } else {
            Some(self.parse_expr()?)
        })
        .at(source))
    }

    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let primary = self.parse_expr_primary()?;
        self.parse_operator_expr(0, primary)
    }

    fn parse_operator_expr(&mut self, precedence: usize, expr: Expr) -> Result<Expr, ParseError> {
        let mut lhs = expr;

        loop {
            let operator = self.input.peek();
            let source = operator.source;
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

            lhs = self.parse_math(lhs, binary_operator, next_precedence, source)?;
        }
    }

    fn parse_expr_primary(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_expr_primary_base()?;
        self.parse_expr_primary_post(expr)
    }

    fn parse_expr_primary_base(&mut self) -> Result<Expr, ParseError> {
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
            TokenKind::Identifier(_) => match self.input.peek_nth(1).kind {
                TokenKind::Namespace => self.parse_enum_member_literal(),
                TokenKind::OpenAngle => self.parse_structure_literal(),
                TokenKind::OpenCurly => {
                    let peek = &self.input.peek_nth(2).kind;

                    if peek.is_extend() || peek.is_colon() {
                        self.parse_structure_literal()
                    } else {
                        let next_three =
                            array_last::<3, 5, _>(self.input.peek_n()).map(|token| &token.kind);

                        match &next_three[..] {
                            [TokenKind::Identifier(_), TokenKind::Colon, ..]
                            | [TokenKind::Newline, TokenKind::Identifier(_), TokenKind::Colon, ..] => {
                                self.parse_structure_literal()
                            }
                            _ => Ok(Expr::new(
                                ExprKind::Variable(self.input.advance().kind.unwrap_identifier()),
                                source,
                            )),
                        }
                    }
                }
                TokenKind::OpenParen => self.parse_call(),
                TokenKind::DeclareAssign => self.parse_declare_assign(),
                _ => Ok(Expr::new(
                    ExprKind::Variable(self.input.advance().kind.unwrap_identifier()),
                    source,
                )),
            },
            TokenKind::Not | TokenKind::BitComplement | TokenKind::Subtract => {
                let operator = match kind {
                    TokenKind::Not => UnaryOperator::Not,
                    TokenKind::BitComplement => UnaryOperator::BitComplement,
                    TokenKind::Subtract => UnaryOperator::Negate,
                    _ => unreachable!(),
                };

                // Eat unary operator
                self.input.advance();

                let inner = self.parse_expr()?;

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
                    _ => {
                        let unexpected = unexpected.to_string();
                        ParseErrorKind::UnexpectedToken { unexpected }
                    }
                },
                source,
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

        let source = self.parse_token(TokenKind::Member, Some("for member expression"))?;
        let field_name = self.parse_identifier(Some("for field name"))?;

        Ok(ExprKind::Member(Box::new(subject), field_name).at(source))
    }

    fn parse_array_access(&mut self, subject: Expr) -> Result<Expr, ParseError> {
        // subject[index]
        //        ^

        let source = self.parse_token(TokenKind::OpenBracket, Some("for array access"))?;

        self.ignore_newlines();
        let index = self.parse_expr()?;
        self.ignore_newlines();

        self.parse_token(TokenKind::CloseBracket, Some("to close array access"))?;

        Ok(ExprKind::ArrayAccess(Box::new(ArrayAccess { subject, index })).at(source))
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
                let field_name = self.parse_identifier(Some("for field name in struct literal"))?;

                self.ignore_newlines();

                let field_value = if dupe {
                    ExprKind::Variable(field_name.clone()).at(source)
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

        let (function_name, source) =
            self.parse_identifier_keep_location(Some("for function call"))?;

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

        Ok(ExprKind::Call(Box::new(Call {
            function_name,
            arguments,
            expected_to_return: None,
        }))
        .at(source))
    }

    fn parse_math(
        &mut self,
        lhs: Expr,
        operator: BinaryOperator,
        operator_precedence: usize,
        source: Source,
    ) -> Result<Expr, ParseError> {
        let rhs = self.parse_math_rhs(operator_precedence)?;

        Ok(match operator {
            BinaryOperator::Basic(basic_operator) => {
                ExprKind::BasicBinaryOperation(Box::new(BasicBinaryOperation {
                    operator: basic_operator,
                    left: lhs,
                    right: rhs,
                }))
            }
            BinaryOperator::ShortCircuiting(short_circuiting_operator) => {
                ExprKind::ShortCircuitingBinaryOperation(Box::new(ShortCircuitingBinaryOperation {
                    operator: short_circuiting_operator,
                    left: lhs,
                    right: rhs,
                }))
            }
        }
        .at(source))
    }

    fn parse_math_rhs(&mut self, operator_precedence: usize) -> Result<Expr, ParseError> {
        // Skip over operator token
        self.input.advance();

        let rhs = self.parse_expr_primary()?;
        let next_operator = self.input.peek();
        let next_precedence = next_operator.kind.precedence();

        if (next_precedence + is_right_associative(next_operator) as usize) >= operator_precedence {
            self.parse_operator_expr(operator_precedence + 1, rhs)
        } else {
            Ok(rhs)
        }
    }

    fn parse_declare_assign(&mut self) -> Result<Expr, ParseError> {
        // variable_name := value
        //               ^

        let (variable_name, source) =
            self.parse_identifier_keep_location(Some("for function call"))?;

        self.parse_token(
            TokenKind::DeclareAssign,
            Some("for variable declaration assignment"),
        )?;
        self.ignore_newlines();

        let value = self.parse_expr()?;

        Ok(ExprKind::DeclareAssign(Box::new(DeclareAssign {
            name: variable_name,
            value,
        }))
        .at(source))
    }

    /// Parses closing '>' brackets of type parameters.
    /// This function may partially consume tokens, so be
    /// aware that any previously peeked tokens may no longer be in
    /// the same lookahead position after calling this function.
    fn parse_type_parameters_close(&mut self) -> Result<(), ParseError> {
        let closer = self.input.advance();

        /// Sub-function for properly handling trailing `=` signs
        /// resulting from partially consuming '>'-like tokens.
        fn merge_trailing_equals<I: Inflow<Token>>(
            parser: &mut Parser<I>,
            closer: &Token,
            column_offset: u32,
        ) {
            if parser.input.eat(TokenKind::Assign) {
                parser
                    .input
                    .unadvance(TokenKind::Equals.at(closer.source.shift_column(column_offset)));
            } else {
                parser
                    .input
                    .unadvance(TokenKind::Assign.at(closer.source.shift_column(column_offset)));
            }
        }

        match &closer.kind {
            TokenKind::GreaterThan => Ok(()),
            TokenKind::RightShift => {
                self.input
                    .unadvance(TokenKind::GreaterThan.at(closer.source.shift_column(1)));
                Ok(())
            }
            TokenKind::LogicalRightShift => {
                self.input
                    .unadvance(TokenKind::RightShift.at(closer.source.shift_column(1)));
                Ok(())
            }
            TokenKind::RightShiftAssign => {
                merge_trailing_equals(self, &closer, 2);

                self.input
                    .unadvance(TokenKind::GreaterThan.at(closer.source.shift_column(1)));
                Ok(())
            }
            TokenKind::LogicalRightShiftAssign => {
                merge_trailing_equals(self, &closer, 3);

                self.input
                    .unadvance(TokenKind::RightShift.at(closer.source.shift_column(1)));
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
        let source = self.input.peek().source;

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
                source,
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

    fn source_here(&mut self) -> Source {
        self.input.peek().source
    }
}

fn is_terminating_token(kind: &TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Comma | TokenKind::CloseParen | TokenKind::CloseBracket | TokenKind::CloseCurly
    )
}

fn is_right_associative(kind: &TokenKind) -> bool {
    matches!(kind, TokenKind::DeclareAssign)
}

// Const evaluation currently isn't strong enough in Rust to write a much better version of this
fn array_last<const LITTLE_N: usize, const BIG_N: usize, T: Copy>(
    big_array: [T; BIG_N],
) -> [T; LITTLE_N] {
    assert!(LITTLE_N <= BIG_N);

    let mut little_array = [const { MaybeUninit::uninit() }; LITTLE_N];

    for i in 0..LITTLE_N {
        little_array[LITTLE_N - i - 1].write(big_array[BIG_N - i - 1]);
    }

    unsafe { MaybeUninit::array_assume_init(little_array) }
}
