use super::{Stmt, StmtKind, Type, TypeKind};
use crate::source_files::Source;

#[derive(Clone, Debug)]
pub struct Block {
    pub stmts: Vec<Stmt>,
}

impl Block {
    pub fn new(stmts: Vec<Stmt>) -> Self {
        Self { stmts }
    }

    pub fn get_result_type(&self, source: Source) -> Type {
        match self.stmts.last().map(|stmt| &stmt.kind) {
            Some(StmtKind::Expr(expr)) => expr.resolved_type.clone(),
            Some(StmtKind::Return(..) | StmtKind::Declaration(..) | StmtKind::Assignment(..))
            | None => TypeKind::Void.at(source),
        }
    }
}
