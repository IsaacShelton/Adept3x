use super::{
    error::ParseErrorKind, AbstractDeclarator, ParseError, Parser, SpecifierQualifierList,
};
use crate::{
    ast::Source,
    c::{
        encoding::Encoding,
        parser::speculate::speculate,
        punctuator::Punctuator,
        token::{CTokenKind, FloatSuffix, Integer},
    },
};

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub enum ExprKind {
    Integer(Integer),
    Float(f64, FloatSuffix),
    StringLiteral(Encoding, String),
    Boolean(bool),
    Nullptr,
    Character(Encoding, String),
    Compound(Vec<Expr>),
    BinaryOperation(Box<BinaryOperation>),
    Ternary(Box<Ternary>),
    Cast(Box<Cast>),
    Subscript(Box<Subscript>),
    Field(Box<Field>),
    PostIncrement(Box<Expr>),
    PostDecrement(Box<Expr>),
    Identifier(String),
    EnumConstant(String, Integer),
}

impl ExprKind {
    pub fn at(self, source: Source) -> Expr {
        Expr { kind: self, source }
    }
}

#[derive(Clone, Debug)]
pub enum BinaryOperator {
    LogicalOr,
    LogicalAnd,
    InclusiveOr,
    ExclusiveOr,
    BitwiseAnd,
    Equals,
    NotEquals,
    LessThan,
    GreaterThan,
    LessThanEq,
    GreaterThanEq,
    LeftShift,
    RightShift,
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulus,
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    ModulusAssign,
    LeftShiftAssign,
    RightShiftAssign,
    BitAndAssign,
    BitXorAssign,
    BitOrAssign,
}

#[derive(Clone, Debug)]
pub struct BinaryOperation {
    pub operator: BinaryOperator,
    pub left: Expr,
    pub right: Expr,
}

#[derive(Clone, Debug)]
pub struct Ternary {
    pub condition: Expr,
    pub when_true: Expr,
    pub when_false: Expr,
}

#[derive(Clone, Debug)]
pub struct Cast {
    pub specializer_qualifiers: SpecifierQualifierList,
    pub abstract_declarator: Option<AbstractDeclarator>,
    pub inner: Expr,
}

#[derive(Clone, Debug)]
pub struct Subscript {
    pub subject: Expr,
    pub subscript: Expr,
    pub source: Source,
}

#[derive(Clone, Debug)]
pub struct Field {
    pub subject: Expr,
    pub field: String,
    pub source: Source,
    pub is_pointer: bool,
}

#[derive(Clone, Debug)]
pub struct Caster {
    pub specializer_qualifiers: SpecifierQualifierList,
    pub abstract_declarator: Option<AbstractDeclarator>,
    pub source: Source,
}

// Implements expression parsing for the C parser
impl<'a> Parser<'a> {
    pub fn parse_expr_singular(&mut self) -> Result<Expr, ParseError> {
        let primary = self.parse_expr_primary()?;
        self.parse_operator_expr(0, primary)
    }

    pub fn parse_expr_multiple(&mut self) -> Result<Expr, ParseError> {
        let source = self.input.peek().source;
        let mut exprs = vec![self.parse_expr_singular()?];

        while self.eat_punctuator(Punctuator::Comma) {
            exprs.push(self.parse_expr_singular()?);
        }

        if exprs.len() == 1 {
            Ok(exprs.drain(..).next().expect("one element"))
        } else {
            Ok(ExprKind::Compound(exprs).at(source))
        }
    }

    fn parse_expr_atom_constant(&mut self) -> Result<Expr, ParseError> {
        let source = self.input.peek().source;

        if let CTokenKind::Integer(integer) = &self.input.peek().kind {
            let integer = integer.clone();
            self.input.advance();
            return Ok(ExprKind::Integer(integer).at(source));
        }

        if let CTokenKind::Float(float, float_suffix) = &self.input.peek().kind {
            let float = float.clone();
            let float_suffix = float_suffix.clone();
            self.input.advance();
            return Ok(ExprKind::Float(float, float_suffix).at(source));
        }

        if let CTokenKind::Identifier(name) = &self.input.peek().kind {
            if let Some(enum_constant) = self.enum_constants.get(name) {
                let name = name.clone();
                let enum_constant = enum_constant.clone();
                self.input.advance();
                return Ok(ExprKind::EnumConstant(name, enum_constant).at(source));
            }
        }

        if let CTokenKind::CharacterConstant(encoding, character) = &self.input.peek().kind {
            let character = character.clone();
            let encoding = encoding.clone();
            self.input.advance();
            return Ok(ExprKind::Character(encoding, character).at(source));
        }

        if self.eat(CTokenKind::TrueKeyword) {
            return Ok(ExprKind::Boolean(true).at(source));
        }

        if self.eat(CTokenKind::FalseKeyword) {
            return Ok(ExprKind::Boolean(false).at(source));
        }

        if self.eat(CTokenKind::NullptrKeyword) {
            return Ok(ExprKind::Nullptr.at(source));
        }

        Err(ParseErrorKind::Misc("Expected expression").at(source))
    }

    fn parse_expr_atom(&mut self) -> Result<Expr, ParseError> {
        let source = self.input.peek().source;

        // Constant
        if let Ok(value) = speculate!(self.input, self.parse_expr_atom_constant()) {
            return Ok(value);
        }

        // String Literal
        if let CTokenKind::StringLiteral(encoding, string) = &self.input.peek().kind {
            let string = string.clone();
            let encoding = encoding.clone();
            self.input.advance();
            return Ok(ExprKind::StringLiteral(encoding, string).at(source));
        }

        // Grouped Expression
        if self.input.peek().is_open_paren() {
            let inner = self.parse_expr_multiple()?;

            if !self.eat_punctuator(Punctuator::CloseParen) {
                return Err(
                    ParseErrorKind::Misc("Expected ')' to close nested expression")
                        .at(self.input.peek().source),
                );
            }

            return Ok(inner);
        }

        // Identifier
        if let Some(identifier) = self.eat_identifier() {
            return Ok(ExprKind::Identifier(identifier).at(source));
        }

        // Generic Selection
        if self.eat(CTokenKind::GenericKeyword) {
            todo!()
        }

        Err(ParseErrorKind::Misc("Expected expression").at(source))
    }

    pub fn parse_expr_primary(&mut self) -> Result<Expr, ParseError> {
        let base = self.parse_expr_primary_base()?;
        self.parse_expr_post(base)
    }

    fn parse_expr_post(&mut self, base: Expr) -> Result<Expr, ParseError> {
        let mut base = base;

        loop {
            if let Some(source) = self.eat_punctuator_source(Punctuator::OpenBracket) {
                let subscript = self.parse_expr_multiple()?;

                if !self.eat_punctuator(Punctuator::CloseBracket) {
                    return Err(ParseErrorKind::Misc("Expected ']' to close subscript")
                        .at(self.input.peek().source));
                }

                base = ExprKind::Subscript(Box::new(Subscript {
                    subject: base,
                    subscript,
                    source,
                }))
                .at(source);
                continue;
            }

            if let Some(source) = self.eat_punctuator_source(Punctuator::Dot) {
                let field = self.eat_identifier().ok_or_else(|| {
                    ParseErrorKind::Misc("Expected field name after '.'").at(source)
                })?;

                base = ExprKind::Field(Box::new(Field {
                    subject: base,
                    field,
                    source,
                    is_pointer: false,
                }))
                .at(source);
                continue;
            }

            if let Some(source) = self.eat_punctuator_source(Punctuator::Arrow) {
                let field = self.eat_identifier().ok_or_else(|| {
                    ParseErrorKind::Misc("Expected field name after '.'").at(source)
                })?;

                base = ExprKind::Field(Box::new(Field {
                    subject: base,
                    field,
                    source,
                    is_pointer: true,
                }))
                .at(source);
                continue;
            }

            if let Some(source) = self.eat_punctuator_source(Punctuator::Increment) {
                base = ExprKind::PostIncrement(Box::new(base)).at(source);
                continue;
            }

            if let Some(source) = self.eat_punctuator_source(Punctuator::Decrement) {
                base = ExprKind::PostDecrement(Box::new(base)).at(source);
                continue;
            }

            if self.eat_open_paren() {
                // Call
                base = todo!();
                continue;
            }

            break;
        }

        Ok(base)
    }

    pub fn parse_expr_primary_base(&mut self) -> Result<Expr, ParseError> {
        // Parse sequence of unary operators and casts

        match &self.input.peek().kind {
            CTokenKind::Punctuator(Punctuator::Ampersand) => todo!(),
            CTokenKind::Punctuator(Punctuator::Multiply) => todo!(),
            CTokenKind::Punctuator(Punctuator::Add) => todo!(),
            CTokenKind::Punctuator(Punctuator::Subtract) => todo!(),
            CTokenKind::Punctuator(Punctuator::BitComplement) => todo!(),
            CTokenKind::Punctuator(Punctuator::Not) => todo!(),
            CTokenKind::Punctuator(Punctuator::Increment) => todo!(),
            CTokenKind::Punctuator(Punctuator::Decrement) => todo!(),
            CTokenKind::SizeofKeyword => todo!(),
            CTokenKind::AlignofKeyword => todo!(),
            _ => (),
        }

        // Is cast?
        if let Ok(caster) = speculate!(self.input, self.parse_caster()) {
            if self.eat_punctuator(Punctuator::OpenCurly) {
                // Compound literal
                return todo!();
            } else {
                // Cast
                return todo!();
            }
        }

        self.parse_expr_atom()
    }

    fn parse_operator_expr(&mut self, precedence: usize, expr: Expr) -> Result<Expr, ParseError> {
        let mut lhs = expr;

        loop {
            let operator = self.input.peek();
            let next_precedence = operator.kind.precedence();

            if is_terminating_token(&operator.kind)
                || (next_precedence + is_right_associative(&operator.kind) as usize) < precedence
            {
                return Ok(lhs);
            }

            // Special case for parsing ternary expressions
            if let CTokenKind::Punctuator(Punctuator::Ternary) = &operator.kind {
                lhs = self.parse_ternary(lhs)?;
                continue;
            }

            let binary_operator = match &operator.kind {
                CTokenKind::Punctuator(Punctuator::LogicalOr) => BinaryOperator::LogicalOr,
                CTokenKind::Punctuator(Punctuator::LogicalAnd) => BinaryOperator::LogicalAnd,
                CTokenKind::Punctuator(Punctuator::BitOr) => BinaryOperator::InclusiveOr,
                CTokenKind::Punctuator(Punctuator::BitXor) => BinaryOperator::ExclusiveOr,
                CTokenKind::Punctuator(Punctuator::Ampersand) => BinaryOperator::BitwiseAnd,
                CTokenKind::Punctuator(Punctuator::DoubleEquals) => BinaryOperator::Equals,
                CTokenKind::Punctuator(Punctuator::NotEquals) => BinaryOperator::NotEquals,
                CTokenKind::Punctuator(Punctuator::LessThan) => BinaryOperator::LessThan,
                CTokenKind::Punctuator(Punctuator::GreaterThan) => BinaryOperator::GreaterThan,
                CTokenKind::Punctuator(Punctuator::LessThanEq) => BinaryOperator::LessThanEq,
                CTokenKind::Punctuator(Punctuator::GreaterThanEq) => BinaryOperator::GreaterThanEq,
                CTokenKind::Punctuator(Punctuator::LeftShift) => BinaryOperator::LeftShift,
                CTokenKind::Punctuator(Punctuator::RightShift) => BinaryOperator::RightShift,
                CTokenKind::Punctuator(Punctuator::Add) => BinaryOperator::Add,
                CTokenKind::Punctuator(Punctuator::Subtract) => BinaryOperator::Subtract,
                CTokenKind::Punctuator(Punctuator::Multiply) => BinaryOperator::Multiply,
                CTokenKind::Punctuator(Punctuator::Divide) => BinaryOperator::Divide,
                CTokenKind::Punctuator(Punctuator::Modulus) => BinaryOperator::Modulus,
                CTokenKind::Punctuator(Punctuator::Assign) => BinaryOperator::Assign,
                CTokenKind::Punctuator(Punctuator::AddAssign) => BinaryOperator::AddAssign,
                CTokenKind::Punctuator(Punctuator::SubtractAssign) => {
                    BinaryOperator::SubtractAssign
                }
                CTokenKind::Punctuator(Punctuator::MultiplyAssign) => {
                    BinaryOperator::MultiplyAssign
                }
                CTokenKind::Punctuator(Punctuator::DivideAssign) => BinaryOperator::DivideAssign,
                CTokenKind::Punctuator(Punctuator::ModulusAssign) => BinaryOperator::ModulusAssign,
                CTokenKind::Punctuator(Punctuator::LeftShiftAssign) => {
                    BinaryOperator::LeftShiftAssign
                }
                CTokenKind::Punctuator(Punctuator::RightShiftAssign) => {
                    BinaryOperator::RightShiftAssign
                }
                CTokenKind::Punctuator(Punctuator::BitAndAssign) => BinaryOperator::BitAndAssign,
                CTokenKind::Punctuator(Punctuator::BitXorAssign) => BinaryOperator::BitXorAssign,
                CTokenKind::Punctuator(Punctuator::BitOrAssign) => BinaryOperator::BitOrAssign,
                _ => return Ok(lhs),
            };

            lhs = self.parse_math(lhs, binary_operator, next_precedence, operator.source)?;
        }
    }

    fn parse_math(
        &mut self,
        lhs: Expr,
        operator: BinaryOperator,
        operator_precedence: usize,
        source: Source,
    ) -> Result<Expr, ParseError> {
        let rhs = self.parse_math_rhs(operator_precedence)?;

        Ok(ExprKind::BinaryOperation(Box::new(BinaryOperation {
            operator,
            left: lhs,
            right: rhs,
        }))
        .at(source))
    }

    fn parse_math_rhs(&mut self, operator_precedence: usize) -> Result<Expr, ParseError> {
        // Skip over operator token
        self.input.advance();

        let rhs = self.parse_expr_primary()?;

        let next_operator = self.input.peek();
        let next_precedence = next_operator.kind.precedence();

        if !((next_precedence + is_right_associative(&next_operator.kind) as usize)
            < operator_precedence)
        {
            self.parse_operator_expr(operator_precedence + 1, rhs)
        } else {
            Ok(rhs)
        }
    }

    fn parse_ternary(&mut self, condition: Expr) -> Result<Expr, ParseError> {
        let source = self.input.peek().source;

        if !self.eat_punctuator(Punctuator::Ternary) {
            return Err(
                ParseErrorKind::Misc("Expected '?' to begin ternary expression").at(source),
            );
        }

        let when_true = self.parse_expr_multiple()?;

        if !self.eat_punctuator(Punctuator::Colon) {
            return Err(
                ParseErrorKind::Misc("Expected ':' during ternary expression")
                    .at(self.input.peek().source),
            );
        }

        let when_false = self.parse_expr_singular()?;

        Ok(ExprKind::Ternary(Box::new(Ternary {
            condition,
            when_true,
            when_false,
        }))
        .at(source))
    }

    fn parse_caster(&mut self) -> Result<Caster, ParseError> {
        let source = self.input.peek().source;

        if !self.eat_open_paren() {
            return Err(ParseErrorKind::Misc("Expected '(' to begin cast").at(source));
        }

        let specializer_qualifiers = self.parse_specifier_qualifier_list()?;
        let abstract_declarator = speculate!(self.input, self.parse_abstract_declarator()).ok();

        if !self.eat_punctuator(Punctuator::CloseParen) {
            return Err(ParseErrorKind::Misc("Expected ')' to close cast").at(source));
        }

        Ok(Caster {
            specializer_qualifiers,
            abstract_declarator,
            source,
        })
    }
}

fn is_terminating_token(kind: &CTokenKind) -> bool {
    match kind {
        CTokenKind::EndOfFile
        | CTokenKind::Punctuator(Punctuator::Comma | Punctuator::CloseParen | Punctuator::Colon) => {
            true
        }
        _ => false,
    }
}

fn is_right_associative(kind: &CTokenKind) -> bool {
    match kind {
        CTokenKind::Punctuator(Punctuator::Ternary)
        | CTokenKind::Punctuator(Punctuator::Assign)
        | CTokenKind::Punctuator(Punctuator::AddAssign)
        | CTokenKind::Punctuator(Punctuator::SubtractAssign)
        | CTokenKind::Punctuator(Punctuator::MultiplyAssign)
        | CTokenKind::Punctuator(Punctuator::DivideAssign)
        | CTokenKind::Punctuator(Punctuator::ModulusAssign)
        | CTokenKind::Punctuator(Punctuator::LeftShiftAssign)
        | CTokenKind::Punctuator(Punctuator::RightShiftAssign)
        | CTokenKind::Punctuator(Punctuator::BitAndAssign)
        | CTokenKind::Punctuator(Punctuator::BitXorAssign)
        | CTokenKind::Punctuator(Punctuator::BitOrAssign) => true,
        _ => false,
    }
}
