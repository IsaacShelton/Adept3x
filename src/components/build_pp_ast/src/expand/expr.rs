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
    error::PreprocessorError,
    parser::{ParseErrorKind, eat_punctuator},
};
use look_ahead::LookAhead;
use pp_ast::{BinaryOperation, BinaryOperator, ConstExpr, Ternary, UnaryOperation, UnaryOperator};
use pp_token::{PreToken, PreTokenKind, Punctuator};
use source_files::Source;
use std::{borrow::Borrow, num::IntErrorKind};

pub struct ExprParser<'a, I>
where
    I: Iterator<Item = &'a PreToken>,
{
    input: LookAhead<I>,
    start_of_line: Source,
}

impl<'a, I: Iterator<Item = &'a PreToken>> ExprParser<'a, I> {
    pub fn new(tokens: I, start_of_line: Source) -> Self {
        Self {
            input: LookAhead::new(tokens),
            start_of_line,
        }
    }

    pub fn parse(tokens: I, start_of_line: Source) -> Result<ConstExpr, PreprocessorError> {
        let mut parser = Self::new(tokens, start_of_line);

        let full_expr = parser.parse_expr()?;
        parser
            .input
            .next()
            .is_none()
            .then_some(full_expr)
            .ok_or_else(|| {
                ParseErrorKind::ExpectedEndOfExpression
                    .at(start_of_line)
                    .into()
            })
    }

    pub fn parse_expr(&mut self) -> Result<ConstExpr, PreprocessorError> {
        let primary = self.parse_expr_primary()?;
        self.parse_operator_expr(0, primary)
    }

    fn parse_expr_primary(&mut self) -> Result<ConstExpr, PreprocessorError> {
        // We don't have any post-fix unary operators,
        // so a primary expression is just the primary base expression
        self.parse_expr_primary_base()
    }

    fn parse_expr_primary_base(&mut self) -> Result<ConstExpr, PreprocessorError> {
        let token = self.input.next();

        match token.map(|token| &token.kind) {
            Some(PreTokenKind::Identifier(name)) => {
                // Undeclared defines are zero (except if identifier is `true` as per C23).
                // (it would've been replaced before expression parsing otherwise)
                Ok(ConstExpr::Constant(if name == "true" { 1 } else { 0 }))
            }
            Some(PreTokenKind::Number(numeric)) => {
                Self::parse_number(numeric, token.unwrap().source)
            }
            Some(PreTokenKind::Punctuator(
                punctuator @ (Punctuator::Not
                | Punctuator::Add
                | Punctuator::Subtract
                | Punctuator::BitComplement),
            )) => self.parse_unary_operation(punctuator),
            Some(PreTokenKind::Punctuator(Punctuator::OpenParen { .. })) => {
                let inner = self.parse_expr()?;
                self.eat_punctuator(Punctuator::CloseParen)?;
                Ok(inner)
            }
            _ => Err(ParseErrorKind::ExpectedExpression
                .at(token
                    .map(|token| token.source)
                    .unwrap_or(self.start_of_line))
                .into()),
        }
    }

    fn parse_unary_operation(
        &mut self,
        punctuator: &Punctuator,
    ) -> Result<ConstExpr, PreprocessorError> {
        let inner = self.parse_expr_primary()?;

        let operator = match punctuator {
            Punctuator::Not => UnaryOperator::Not,
            Punctuator::Add => UnaryOperator::Positive,
            Punctuator::Subtract => UnaryOperator::Negative,
            Punctuator::BitComplement => UnaryOperator::BitComplement,
            _ => unreachable!(),
        };

        Ok(ConstExpr::UnaryOperation(Box::new(UnaryOperation {
            operator,
            inner,
        })))
    }

    fn parse_number(number: &str, source: Source) -> Result<ConstExpr, PreprocessorError> {
        // Remove trailing 'L' if present
        let number = number.strip_suffix("L").unwrap_or(number);

        // Parse number depending on prefix
        if let Some(hex_digits) = number
            .strip_prefix("0x")
            .or_else(|| number.strip_prefix("0X"))
        {
            Self::parse_number_radix(hex_digits, 16)
        } else if number.starts_with("0") {
            Self::parse_number_radix(number, 8)
        } else {
            Self::parse_number_radix(number, 10)
        }
        .map_err(|err| err.at(source).into())
    }

    fn parse_number_radix(number: &str, radix: u32) -> Result<ConstExpr, ParseErrorKind> {
        match i64::from_str_radix(number, radix) {
            Ok(value) => Ok(ConstExpr::Constant(value)),
            Err(error) if *error.kind() == IntErrorKind::PosOverflow => {
                match u64::from_str_radix(number, radix) {
                    Ok(value) => Ok(ConstExpr::Constant(value as i64)),
                    Err(_) => Err(ParseErrorKind::BadInteger),
                }
            }
            Err(_) => Err(ParseErrorKind::BadInteger),
        }
    }

    fn parse_ternary(&mut self, condition: ConstExpr) -> Result<ConstExpr, PreprocessorError> {
        self.eat_punctuator(Punctuator::Ternary)?;
        let when_true = self.parse_expr()?;
        self.eat_punctuator(Punctuator::Colon)?;
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
    ) -> Result<ConstExpr, PreprocessorError> {
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
    ) -> Result<ConstExpr, PreprocessorError> {
        let rhs = self.parse_math_rhs(operator_precedence)?;

        Ok(ConstExpr::BinaryOperation(Box::new(BinaryOperation {
            operator,
            left: lhs,
            right: rhs,
        })))
    }

    fn parse_math_rhs(
        &mut self,
        operator_precedence: usize,
    ) -> Result<ConstExpr, PreprocessorError> {
        // Skip over operator token
        self.input.next();

        let rhs = self.parse_expr_primary()?;

        let next_operator = match self.input.peek() {
            Some(operator) => operator,
            None => return Ok(rhs),
        };

        let next_precedence = next_operator.kind.precedence();

        if (next_precedence + is_right_associative(&next_operator.kind) as usize)
            >= operator_precedence
        {
            self.parse_operator_expr(operator_precedence + 1, rhs)
        } else {
            Ok(rhs)
        }
    }

    fn eat_punctuator(
        &mut self,
        expected: impl Borrow<Punctuator>,
    ) -> Result<(), PreprocessorError> {
        let source = self
            .input
            .peek()
            .map_or(self.start_of_line, |token| token.source);
        eat_punctuator(&mut self.input, expected, source)
    }
}

fn is_terminating_token(kind: &PreTokenKind) -> bool {
    matches!(
        kind,
        PreTokenKind::Punctuator(Punctuator::Comma | Punctuator::CloseParen | Punctuator::Colon,)
    )
}

fn is_right_associative(kind: &PreTokenKind) -> bool {
    matches!(kind, PreTokenKind::Punctuator(Punctuator::Ternary))
}
