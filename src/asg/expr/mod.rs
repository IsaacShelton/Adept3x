mod array_access;
mod binary;
mod call;
mod cast;
mod cast_from;
mod conditional;
mod decl_assign;
mod enum_member;
mod global_variable;
mod kind;
mod member;
mod poly_call;
mod struct_literal;
mod typed;
mod unary;
mod variable;
mod while_loop;

use crate::source_files::Source;
pub use array_access::*;
pub use binary::*;
pub use call::*;
pub use cast::*;
pub use cast_from::*;
pub use conditional::*;
use core::hash::Hash;
pub use decl_assign::*;
pub use enum_member::*;
pub use global_variable::*;
pub use kind::*;
pub use member::*;
pub use struct_literal::*;
pub use typed::*;
pub use unary::*;
pub use variable::*;
pub use while_loop::*;

#[derive(Clone, Debug)]
pub struct Expr {
    pub kind: ExprKind,
    pub source: Source,
}

impl Hash for Expr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.kind.hash(state)
    }
}

impl PartialEq for Expr {
    fn eq(&self, other: &Self) -> bool {
        self.kind.eq(&other.kind)
    }
}

impl Eq for Expr {}

impl Expr {
    pub fn new(kind: ExprKind, source: Source) -> Self {
        Self { kind, source }
    }
}
