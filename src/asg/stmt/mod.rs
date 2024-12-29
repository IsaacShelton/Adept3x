mod assignment;
mod declaration;

use super::{Expr, TypedExpr};
use crate::source_files::Source;
pub use assignment::*;
use core::hash::Hash;
pub use declaration::*;
use derive_more::Unwrap;

#[derive(Clone, Debug)]
pub struct Stmt {
    pub kind: StmtKind,
    pub source: Source,
}

impl Hash for Stmt {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.kind.hash(state)
    }
}

impl PartialEq for Stmt {
    fn eq(&self, other: &Self) -> bool {
        self.kind.eq(&other.kind)
    }
}

impl Eq for Stmt {}

impl Stmt {
    pub fn new(kind: StmtKind, source: Source) -> Self {
        Self { kind, source }
    }
}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Unwrap)]
pub enum StmtKind {
    Return(Option<Expr>),
    Expr(TypedExpr),
    Declaration(Declaration),
    Assignment(Assignment),
}
