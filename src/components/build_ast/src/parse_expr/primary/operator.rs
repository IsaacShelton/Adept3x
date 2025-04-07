use super::{Parser, is_right_associative, is_terminating_token};
use crate::error::ParseError;
use ast::{
    BasicBinaryOperation, BasicBinaryOperator, BinaryOperator, Expr, ExprKind, Language,
    ShortCircuitingBinaryOperation, ShortCircuitingBinaryOperator,
};
use infinite_iterator::InfinitePeekable;
use source_files::Source;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_operator_expr(
        &mut self,
        precedence: usize,
        expr: Expr,
    ) -> Result<Expr, ParseError> {
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
                TokenKind::BitAnd => BasicBinaryOperator::BitwiseAnd.into(),
                TokenKind::BitOr => BasicBinaryOperator::BitwiseOr.into(),
                TokenKind::BitXor => BasicBinaryOperator::BitwiseXor.into(),
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
                    language: Language::Adept,
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
}
