use smallvec::SmallVec;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct StaticScope {
    pub names: HashMap<String, SmallVec<[Decl; 4]>>,
}

#[derive(Debug)]
pub enum TypeRef {
    Struct(ast_workspace::StructRef),
    Enum(ast_workspace::EnumRef),
    Alias(ast_workspace::TypeAliasRef),
    Trait(ast_workspace::TraitRef),
}

#[derive(Debug)]
pub enum Decl {
    Global(ast_workspace::GlobalRef),
    Func(ast_workspace::FuncRef),
    Type(TypeRef),
    Impl(ast_workspace::ImplRef),
    Namespace(),
    ExprAlias(ast_workspace::ExprAliasRef),
}
