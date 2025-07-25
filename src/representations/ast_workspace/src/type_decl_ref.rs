use crate::{EnumRef, StructRef, TraitRef, TypeAliasRef};
use ast::{Enum, Struct, Trait, TypeAlias};

/// An abstract reference to an AST type declaration
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum TypeDeclRef {
    Struct(StructRef),
    Enum(EnumRef),
    Alias(TypeAliasRef),
    Trait(TraitRef),
}

pub enum TypeDecl<'a> {
    Struct(&'a Struct),
    Enum(&'a Enum),
    Alias(&'a TypeAlias),
    Trait(&'a Trait),
}
