use super::{
    error::ParseErrorKind, AbstractDeclarator, ParseError, Parser, SpecifierQualifierList,
};
use crate::{
    ast::Source,
    c::{
        parser::speculate::speculate,
        punctuator::Punctuator,
        token::{CTokenKind, Integer},
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
    Compound(Vec<Expr>),
    BinaryOperation(Box<BinaryOperation>),
    Ternary(Box<Ternary>),
    Cast(Box<Cast>),
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

// Implements expression parsing for the C parser
impl<'a> Parser<'a> {
    // NOTE: Corresponds to `assignment-expression` in the spec
    pub fn parse_assignment_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_singular_expr()
    }

    // NOTE: Corresponds to `expression` in the spec
    // This means compound expressions are supported! e.g. `1, 2, 3` is an expression!
    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        let source = self.input.peek().source;
        let mut exprs = vec![self.parse_singular_expr()?];

        while self.eat_punctuator(Punctuator::Comma) {
            exprs.push(self.parse_singular_expr()?);
        }

        if exprs.len() == 1 {
            Ok(exprs.drain(..).next().expect("one element"))
        } else {
            Ok(ExprKind::Compound(exprs).at(source))
        }
    }

    fn parse_singular_expr(&mut self) -> Result<Expr, ParseError> {
        let primary = self.parse_expr_primary()?;
        self.parse_operator_expr(0, primary)
    }

    fn parse_expr_primary(&mut self) -> Result<Expr, ParseError> {
        let _base = self.parse_cast_expr();
        todo!("post-fix operators")
    }

    fn parse_cast_expr(&mut self) -> Result<Expr, ParseError> {
        if let Ok(cast) = speculate!(self.input, self.parse_cast()) {
            return Ok(cast);
        }

        self.parse_unary_expr()
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, ParseError> {
        if self.eat_punctuator(Punctuator::Ampersand) {
            let inner = self.parse_cast_expr()?;
            return Ok(todo!());
        }

        if self.eat_punctuator(Punctuator::Multiply) {
            let inner = self.parse_cast_expr()?;
            return Ok(todo!());
        }

        if self.eat_punctuator(Punctuator::Add) {
            let inner = self.parse_cast_expr()?;
            return Ok(todo!());
        }

        if self.eat_punctuator(Punctuator::Subtract) {
            let inner = self.parse_cast_expr()?;
            return Ok(todo!());
        }

        if self.eat_punctuator(Punctuator::BitComplement) {
            let inner = self.parse_cast_expr()?;
            return Ok(todo!());
        }

        if self.eat_punctuator(Punctuator::Not) {
            let inner = self.parse_cast_expr()?;
            return Ok(todo!());
        }

        if self.eat_punctuator(Punctuator::Increment) {
            let inner = self.parse_unary_expr()?;
            return Ok(todo!());
        }

        if self.eat_punctuator(Punctuator::Decrement) {
            let inner = self.parse_unary_expr()?;
            return Ok(todo!());
        }

        if self.eat(CTokenKind::SizeofKeyword) {
            if self.eat_open_paren() {
                todo!();
            } else {
                todo!();
            }

            let inner = self.parse_unary_expr()?;
            return Ok(todo!());
        }

        if self.eat(CTokenKind::AlignofKeyword) {
            todo!();
            return Ok(todo!());
        }

        self.parse_postfix_expr()
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
            return Err(ParseErrorKind::Misc("Expected '?' for ternary expression").at(source));
        }

        let when_true = self.parse_expr()?;

        if !self.eat_punctuator(Punctuator::Colon) {
            return Err(ParseErrorKind::Misc("Expected '?' for ternary expression")
                .at(self.input.peek().source));
        }

        let when_false = self.parse_expr()?;

        Ok(ExprKind::Ternary(Box::new(Ternary {
            condition,
            when_true,
            when_false,
        }))
        .at(source))
    }

    fn parse_cast(&mut self) -> Result<Expr, ParseError> {
        let source = self.input.peek().source;

        if !self.eat_open_paren() {
            return Err(ParseErrorKind::Misc("Expected '(' to begin cast").at(source));
        }

        let specializer_qualifiers = self.parse_specifier_qualifier_list()?;
        let abstract_declarator = speculate!(self.input, self.parse_abstract_declarator()).ok();

        if !self.eat_punctuator(Punctuator::CloseParen) {
            return Err(ParseErrorKind::Misc("Expected ')' to close cast").at(source));
        }

        let inner = self.parse_expr_primary()?;

        Ok(ExprKind::Cast(Box::new(Cast {
            specializer_qualifiers,
            abstract_declarator,
            inner,
        }))
        .at(source))
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
