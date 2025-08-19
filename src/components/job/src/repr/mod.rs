mod compiler;
mod decl;
mod decl_head_set;
mod decl_set;
mod enum_body;
mod evaluated;
mod func_body;
mod func_head;
mod params;
mod struct_body;
mod trait_body;
mod ty;
mod type_alias_body;
mod type_head;
mod variables;

use ast_workspace::TypeDeclRef;
pub use compiler::*;
pub use decl::*;
pub use decl_head_set::*;
pub use decl_set::*;
pub use enum_body::*;
pub use evaluated::*;
pub use func_body::*;
pub use func_head::*;
pub use params::*;
pub use struct_body::*;
pub use trait_body::*;
pub use ty::*;
pub use type_alias_body::*;
pub use type_head::*;
pub use variables::*;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AmbiguousType;

pub type FindTypeResult = Result<Option<TypeDeclRef>, AmbiguousType>;
