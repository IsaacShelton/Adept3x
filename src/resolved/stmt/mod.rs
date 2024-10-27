mod assignment;
mod declaration;

use super::{Expr, TypedExpr};
use crate::source_files::Source;
pub use assignment::*;
pub use declaration::*;
use derive_more::Unwrap;

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub source: Source,
}

impl Stmt {
    pub fn new(kind: StmtKind, source: Source) -> Self {
        Self { kind, source }
    }
}

#[derive(Clone, Debug, Unwrap)]
pub enum StmtKind {
    Return(Option<Expr>),
    Expr(TypedExpr),
    Declaration(Declaration),
    Assignment(Assignment),
}
