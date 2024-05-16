/*
    Constant expression parser for C preprocessor #if-like directives.

    Each expression must be re-parsed per environment, since all `#define`s
    must be replaced before parsing the expression, which may include additions that aren't strictly
    on expression level boundries.

    For example:

    ````
    #include <stdio.h>

    #define X 10
    #define Y 10

    #define A X
    #define B == Y

    int main(){
        #if A B
        printf("Hello World\n");
        #endif
        return 0;
    }
    ```

    Should result in:

    ```
    #include <stdio.h>

    int main(){
        printf("Hello World\n");
        return 0;
    }
    ```

    Cursed, I know.
*/

use crate::{
    c::preprocessor::{
        ast::{BinaryOperation, BinaryOperator, ConstExpr, Ternary},
        pre_token::{PreToken, PreTokenKind, Punctuator},
        ParseError,
    },
    look_ahead::LookAhead,
};
use std::num::IntErrorKind;

pub struct ExprParser<'a, I>
where
    I: Iterator<Item = &'a PreToken>,
{
    input: LookAhead<I>,
}

impl<'a, I: Iterator<Item = &'a PreToken>> ExprParser<'a, I> {
    pub fn new(tokens: I) -> Self {
        Self {
            input: LookAhead::new(tokens),
        }
    }

    pub fn parse(tokens: I) -> Result<ConstExpr, ParseError> {
        let mut parser = Self::new(tokens);
        parser.parse_expr()
    }

    pub fn parse_expr(&mut self) -> Result<ConstExpr, ParseError> {
        let primary = self.parse_expr_primary()?;
        self.parse_operator_expr(0, primary)
    }

    fn parse_expr_primary(&mut self) -> Result<ConstExpr, ParseError> {
        // We don't have any post-fix unary operators,
        // so a primary expression is just the primary base expression
        self.parse_expr_primary_base()
    }

    fn parse_expr_primary_base(&mut self) -> Result<ConstExpr, ParseError> {
        let token = self.input.peek();

        // TODO: Clean this up
        match token.map(|token| &token.kind) {
            Some(PreTokenKind::Identifier(name)) => {
                // Undeclared defines are zero (except if identifier is `true` as per C23).
                // (it would've been replaced before expression parsing otherwise)
                self.input.next().unwrap();

                Ok(ConstExpr::Constant(if name == "true" { 1 } else { 0 }))
            }
            Some(PreTokenKind::Number(numeric)) => {
                self.input.next().unwrap();
                Self::parse_number(numeric)
            }
            Some(PreTokenKind::Punctuator(Punctuator::OpenParen { .. })) => {
                // Eat '('
                self.input.next().unwrap();

                let inner = self.parse_expr()?;

                // Eat ')'
                match self.input.peek().map(|pre_token| &pre_token.kind) {
                    Some(PreTokenKind::Punctuator(Punctuator::CloseParen)) => {
                        self.input.next().unwrap();
                    }
                    _ => return Err(ParseError::ExpectedCloseParen),
                }

                Ok(inner)
            }
            _ => Err(ParseError::ExpectedExpression),
        }
    }

    fn parse_number(number: &str) -> Result<ConstExpr, ParseError> {
        if number.starts_with("0x") || number.starts_with("0X") {
            Self::parse_number_radix(&number[..2], 16)
        } else if number.starts_with("0") {
            Self::parse_number_radix(number, 8)
        } else {
            Self::parse_number_radix(number, 10)
        }
    }

    fn parse_number_radix(number: &str, radix: u32) -> Result<ConstExpr, ParseError> {
        match i64::from_str_radix(number, radix) {
            Ok(value) => Ok(ConstExpr::Constant(value)),
            Err(error) if *error.kind() == IntErrorKind::PosOverflow => {
                match u64::from_str_radix(number, radix) {
                    Ok(value) => Ok(ConstExpr::Constant(value as i64)),
                    Err(_) => Err(ParseError::BadInteger),
                }
            }
            Err(_) => Err(ParseError::BadInteger),
        }
    }

    fn parse_ternary(&mut self, condition: ConstExpr) -> Result<ConstExpr, ParseError> {
        // Eat '?'
        self.input.next().unwrap();

        let when_true = self.parse_expr()?;

        // Eat ':'
        // TODO: CLEANUP: Clean this up messy part
        match self.input.peek().map(|pre_token| &pre_token.kind) {
            Some(PreTokenKind::Punctuator(Punctuator::Colon)) => _ = self.input.next().unwrap(),
            _ => return Err(ParseError::ExpectedColon),
        }

        let when_false = self.parse_expr()?;

        Ok(ConstExpr::Ternary(Box::new(Ternary {
            condition,
            when_true,
            when_false,
        })))
    }

    fn parse_operator_expr(
        &mut self,
        precedence: usize,
        expr: ConstExpr,
    ) -> Result<ConstExpr, ParseError> {
        let mut lhs = expr;

        loop {
            let operator = match self.input.peek() {
                Some(operator) => operator,
                None => return Ok(lhs),
            };

            let next_precedence = operator.kind.precedence();

            if is_terminating_token(&operator.kind)
                || (next_precedence + is_right_associative(&operator.kind) as usize) < precedence
            {
                return Ok(lhs);
            }

            // Special case for parsing ternary expressions
            if let PreTokenKind::Punctuator(Punctuator::Ternary) = &operator.kind {
                lhs = self.parse_ternary(lhs)?;
                continue;
            }

            let binary_operator = match &operator.kind {
                PreTokenKind::Punctuator(Punctuator::LogicalOr) => BinaryOperator::LogicalOr,
                PreTokenKind::Punctuator(Punctuator::LogicalAnd) => BinaryOperator::LogicalAnd,
                PreTokenKind::Punctuator(Punctuator::BitOr) => BinaryOperator::InclusiveOr,
                PreTokenKind::Punctuator(Punctuator::BitXor) => BinaryOperator::ExclusiveOr,
                PreTokenKind::Punctuator(Punctuator::Ampersand) => BinaryOperator::BitwiseAnd,
                PreTokenKind::Punctuator(Punctuator::DoubleEquals) => BinaryOperator::Equals,
                PreTokenKind::Punctuator(Punctuator::NotEquals) => BinaryOperator::NotEquals,
                PreTokenKind::Punctuator(Punctuator::LessThan) => BinaryOperator::LessThan,
                PreTokenKind::Punctuator(Punctuator::GreaterThan) => BinaryOperator::GreaterThan,
                PreTokenKind::Punctuator(Punctuator::LessThanEq) => BinaryOperator::LessThanEq,
                PreTokenKind::Punctuator(Punctuator::GreaterThanEq) => {
                    BinaryOperator::GreaterThanEq
                }
                PreTokenKind::Punctuator(Punctuator::LeftShift) => BinaryOperator::LeftShift,
                PreTokenKind::Punctuator(Punctuator::RightShift) => BinaryOperator::RightShift,
                PreTokenKind::Punctuator(Punctuator::Add) => BinaryOperator::Add,
                PreTokenKind::Punctuator(Punctuator::Subtract) => BinaryOperator::Subtract,
                PreTokenKind::Punctuator(Punctuator::Multiply) => BinaryOperator::Multiply,
                PreTokenKind::Punctuator(Punctuator::Divide) => BinaryOperator::Divide,
                PreTokenKind::Punctuator(Punctuator::Modulus) => BinaryOperator::Modulus,
                _ => return Ok(lhs),
            };

            lhs = self.parse_math(lhs, binary_operator, next_precedence)?;
        }
    }

    fn parse_math(
        &mut self,
        lhs: ConstExpr,
        operator: BinaryOperator,
        operator_precedence: usize,
    ) -> Result<ConstExpr, ParseError> {
        let rhs = self.parse_math_rhs(operator_precedence)?;

        Ok(ConstExpr::BinaryOperation(Box::new(BinaryOperation {
            operator,
            left: lhs,
            right: rhs,
        })))
    }

    fn parse_math_rhs(&mut self, operator_precedence: usize) -> Result<ConstExpr, ParseError> {
        // Skip over operator token
        self.input.next();

        let rhs = self.parse_expr_primary()?;

        let next_operator = match self.input.peek() {
            Some(operator) => operator,
            None => return Ok(rhs),
        };

        let next_precedence = next_operator.kind.precedence();

        if !((next_precedence + is_right_associative(&next_operator.kind) as usize)
            < operator_precedence)
        {
            self.parse_operator_expr(operator_precedence + 1, rhs)
        } else {
            Ok(rhs)
        }
    }
}

fn is_terminating_token(kind: &PreTokenKind) -> bool {
    match kind {
        PreTokenKind::Punctuator(
            Punctuator::Comma | Punctuator::CloseParen | Punctuator::Colon,
        ) => true,
        _ => false,
    }
}

fn is_right_associative(kind: &PreTokenKind) -> bool {
    match kind {
        PreTokenKind::Punctuator(Punctuator::Ternary) => true,
        _ => false,
    }
}
