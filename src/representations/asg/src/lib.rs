mod block;
mod datatype;
mod destination;
mod enumeration;
mod error;
mod expr;
mod func;
mod generic_trait_ref;
mod global;
mod helper_expr;
mod human_name;
mod impl_params;
mod implementation;
mod name;
mod poly_catalog;
mod poly_recipe;
mod poly_resolver;
mod poly_value;
mod stmt;
mod structure;
mod trait_constraint;
mod type_alias;
mod type_decl;
mod variable_storage;

pub use self::variable_storage::VariableStorageKey;
use ast_workspace::AstWorkspace;
pub use block::*;
pub use datatype::*;
pub use destination::*;
pub use enumeration::*;
pub use error::*;
pub use expr::*;
pub use func::*;
pub use generic_trait_ref::*;
pub use global::*;
pub use helper_expr::*;
pub use human_name::*;
pub use impl_params::*;
pub use implementation::*;
pub use name::ResolvedName;
pub use poly_catalog::*;
pub use poly_recipe::PolyRecipe;
pub use poly_resolver::{IntoPolyRecipeResolver, PolyRecipeResolver};
pub use poly_value::*;
use slotmap::{SlotMap, new_key_type};
use source_files::SourceFiles;
pub use stmt::*;
pub use structure::*;
pub use trait_constraint::*;
pub use type_alias::TypeAlias;
pub use type_decl::*;
pub use variable_storage::*;

new_key_type! {
    pub struct FuncRef;
    pub struct GlobalRef;
    pub struct StructRef;
    pub struct EnumRef;
    pub struct TypeAliasRef;
    pub struct TraitRef;
    pub struct ImplRef;
}

#[derive(Clone, Debug)]
pub struct Asg<'a> {
    pub source_files: &'a SourceFiles,
    pub entry_point: Option<FuncRef>,
    pub funcs: SlotMap<FuncRef, Func>,
    pub structs: SlotMap<StructRef, Struct>,
    pub globals: SlotMap<GlobalRef, Global>,
    pub enums: SlotMap<EnumRef, Enum>,
    pub type_aliases: SlotMap<TypeAliasRef, TypeAlias>,
    pub traits: SlotMap<TraitRef, Trait>,
    pub impls: SlotMap<ImplRef, Impl>,
    pub workspace: &'a AstWorkspace<'a>,
}

impl<'a> Asg<'a> {
    pub fn new(source_files: &'a SourceFiles, workspace: &'a AstWorkspace) -> Self {
        Self {
            source_files,
            entry_point: None,
            funcs: SlotMap::with_key(),
            structs: SlotMap::with_key(),
            globals: SlotMap::with_key(),
            enums: SlotMap::with_key(),
            type_aliases: SlotMap::with_key(),
            traits: SlotMap::with_key(),
            impls: SlotMap::with_key(),
            workspace,
        }
    }
}
