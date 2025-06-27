use super::{Assignment, Declaration, Stmt};
use crate::Expr;
use source_files::Source;

#[derive(Clone, Debug)]
pub enum StmtKind {
    Return(Option<Expr>),
    Expr(Expr),
    Declaration(Box<Declaration>),
    Assignment(Box<Assignment>),
    Label(String),
    // NOTE: This should eventually be an Expr to support computed gotos
    Goto(String),
}

impl StmtKind {
    pub fn at(self, source: Source) -> Stmt {
        Stmt { kind: self, source }
    }
}
