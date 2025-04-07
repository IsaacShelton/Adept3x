use super::{super::error::ParseError, Parser};
use ast::{Assignment, BasicBinaryOperator, Expr, Stmt, StmtKind};
use inflow::Inflow;
use token::{Token, TokenKind};

impl<'a, I: Inflow<Token>> Parser<'a, I> {
    pub fn parse_assignment(&mut self, destination: Expr) -> Result<Stmt, ParseError> {
        let source = self.input.peek().source;

        let operator = match self.input.advance().kind {
            TokenKind::Assign => None,
            TokenKind::AddAssign => Some(BasicBinaryOperator::Add),
            TokenKind::SubtractAssign => Some(BasicBinaryOperator::Subtract),
            TokenKind::MultiplyAssign => Some(BasicBinaryOperator::Multiply),
            TokenKind::DivideAssign => Some(BasicBinaryOperator::Divide),
            TokenKind::ModulusAssign => Some(BasicBinaryOperator::Modulus),
            TokenKind::BitAndAssign => Some(BasicBinaryOperator::BitwiseAnd),
            TokenKind::BitOrAssign => Some(BasicBinaryOperator::BitwiseOr),
            TokenKind::BitXorAssign => Some(BasicBinaryOperator::BitwiseXor),
            TokenKind::LeftShiftAssign => Some(BasicBinaryOperator::LeftShift),
            TokenKind::RightShiftAssign => Some(BasicBinaryOperator::RightShift),
            TokenKind::LogicalLeftShiftAssign => Some(BasicBinaryOperator::LogicalLeftShift),
            TokenKind::LogicalRightShiftAssign => Some(BasicBinaryOperator::LogicalRightShift),
            got => {
                return Err(ParseError::expected(
                    "(an assignment operator)",
                    Some("for assignment"),
                    got.at(source),
                ));
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
}
