use super::TypeRef;

/// A symbol declaration
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Decl {
    Global(ast_workspace::GlobalRef),
    Func(ast_workspace::FuncRef),
    Type(TypeRef),
    Impl(ast_workspace::ImplRef),
    Namespace(ast_workspace::NamespaceRef),
    ExprAlias(ast_workspace::ExprAliasRef),
}
