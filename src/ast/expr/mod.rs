mod array_access;
mod binary;
mod call;
mod conditional;
mod constant;
mod declare_assign;
mod enum_member;
mod integer;
mod interpreter_syscall;
mod kind;
mod structure_literal;
mod unary;
mod while_loop;

use crate::source_files::Source;
pub use array_access::*;
pub use binary::*;
pub use call::*;
pub use conditional::*;
#[allow(unused_imports)]
pub use constant::*;
pub use declare_assign::*;
pub use enum_member::*;
pub use integer::*;
pub use interpreter_syscall::*;
pub use kind::*;
pub use structure_literal::*;
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
}

// Make sure ExprKind doesn't accidentally become huge
const _: () = assert!(std::mem::size_of::<ExprKind>() <= 48);
