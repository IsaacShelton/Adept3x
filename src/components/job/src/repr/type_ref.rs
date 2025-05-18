/// An abstract reference to an AST type declaration

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeRef {
    Struct(ast_workspace::StructRef),
    Enum(ast_workspace::EnumRef),
    Alias(ast_workspace::TypeAliasRef),
    Trait(ast_workspace::TraitRef),
}
