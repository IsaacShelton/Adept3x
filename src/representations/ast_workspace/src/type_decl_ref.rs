use crate::{EnumRef, StructRef, TraitRef, TypeAliasRef};

/// An abstract reference to an AST type declaration
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeDeclRef {
    Struct(StructRef),
    Enum(EnumRef),
    Alias(TypeAliasRef),
    Trait(TraitRef),
}
