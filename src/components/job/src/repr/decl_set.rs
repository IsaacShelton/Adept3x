use super::Decl;
use ast_workspace::TypeDeclRef;
use std_ext::SmallVec4;

/// A group of declarations under the same name
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct DeclSet(SmallVec4<Decl>);

impl<'env> DeclSet {
    pub fn push_unique(&mut self, decl: Decl) {
        self.0.push(decl);
    }

    pub fn type_decls(&self) -> impl Iterator<Item = TypeDeclRef> {
        self.0.iter().filter_map(|decl| match decl {
            Decl::Type(type_decl_ref) => Some(*type_decl_ref),
            _ => None,
        })
    }

    pub fn func_decls(&self) -> impl Iterator<Item = ast_workspace::FuncRef> {
        self.0.iter().filter_map(|decl| match decl {
            Decl::Func(func_ref) => Some(*func_ref),
            _ => None,
        })
    }
}
