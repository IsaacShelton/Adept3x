mod array_access;
mod binary;
mod call;
mod conditional;
mod constant;
mod declare_assign;
mod integer;
mod interpreter_syscall;
mod kind;
mod static_member;
mod struct_literal;
mod unary;
mod while_loop;

use super::{Stmt, StmtKind};
pub use array_access::*;
pub use binary::*;
pub use call::*;
pub use conditional::*;
#[allow(unused_imports)]
pub use constant::*;
pub use declare_assign::*;
pub use integer::*;
pub use interpreter_syscall::*;
pub use kind::*;
use source_files::Source;
pub use static_member::*;
pub use struct_literal::*;
pub use unary::*;
pub use while_loop::*;

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub source: Source,
}

impl Expr {
    pub fn new(kind: ExprKind, source: Source) -> Self {
        Self { kind, source }
    }

    pub fn stmt(self) -> Stmt {
        let source = self.source;
        return StmtKind::Expr(self).at(source);
    }
}

// Make sure ExprKind doesn't accidentally become huge
const _: () = assert!(std::mem::size_of::<ExprKind>() <= 48);
