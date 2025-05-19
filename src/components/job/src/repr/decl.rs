use ast_workspace::TypeDeclRef;

/// A symbol declaration
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Decl {
    Global(ast_workspace::GlobalRef),
    Func(ast_workspace::FuncRef),
    Type(TypeDeclRef),
    Impl(ast_workspace::ImplRef),
    Namespace(ast_workspace::NamespaceRef),
    ExprAlias(ast_workspace::ExprAliasRef),
}
