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
        if let Some(stmt) = self.stmts.last() {
            match &stmt.kind {
                StmtKind::Return(..) => None,
                StmtKind::Expr(expr) => Some(expr.resolved_type.clone()),
                StmtKind::Declaration(..) => None,
                StmtKind::Assignment(..) => None,
            }
        } else {
            None
        }
        .unwrap_or(TypeKind::Void.at(source))
    }
}
