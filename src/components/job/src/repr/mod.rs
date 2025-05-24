mod decl;
mod decl_scope;
mod decl_scope_origin;
mod decl_scope_ref;
mod decl_set;
mod enum_body;
mod struct_body;
mod ty;
mod type_head;

use ast_workspace::TypeDeclRef;
pub use decl::*;
pub use decl_scope::*;
pub use decl_scope_origin::*;
pub use decl_scope_ref::*;
pub use decl_set::*;
pub use enum_body::*;
pub use struct_body::*;
pub use ty::*;
pub use type_head::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AmbiguousType;

pub type FindTypeResult = Result<Option<TypeDeclRef>, AmbiguousType>;
