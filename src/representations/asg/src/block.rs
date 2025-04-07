use super::{Stmt, StmtKind, Type, TypeKind};
use source_files::Source;

#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

impl Block {
    pub fn new(stmts: Vec<Stmt>) -> Self {
        Self { stmts }
    }

    pub fn get_result_type(&self, source: Source) -> Type {
        match self.stmts.last().map(|stmt| &stmt.kind) {
            Some(StmtKind::Expr(expr)) => expr.ty.clone(),
            Some(StmtKind::Return(..) | StmtKind::Declaration(..) | StmtKind::Assignment(..))
            | None => TypeKind::Void.at(source),
        }
    }
}
