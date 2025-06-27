use super::{super::error::ParseError, Parser};
use ast::{ExprKind, Stmt, StmtKind};
use infinite_iterator::InfinitePeekable;
use token::{Token, TokenKind};

impl<'a, I: InfinitePeekable<Token>> Parser<'a, I> {
    pub fn parse_goto(&mut self) -> Result<Stmt, ParseError> {
        // goto VALUE
        //        ^

        let source = self.input.here();

        self.input
            .expect(TokenKind::GotoKeyword, "for goto statement")?;

        let expr = self.parse_expr()?;

        let ExprKind::LabelLiteral(label_name) = expr.kind else {
            return Err(ParseError::other(
                "Computed 'goto's are not supported yet",
                expr.source,
            ));
        };

        Ok(StmtKind::Goto(label_name).at(source))
    }
}
