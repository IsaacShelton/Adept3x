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
use arena::{Arena, Idx, new_id_with_niche};
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
use source_files::SourceFiles;
pub use stmt::*;
pub use structure::*;
pub use trait_constraint::*;
pub use type_alias::TypeAlias;
pub use type_decl::*;
pub use variable_storage::*;

new_id_with_niche!(FuncId, u64);
new_id_with_niche!(StructId, u64);
new_id_with_niche!(EnumId, u64);
new_id_with_niche!(GlobalId, u64);
new_id_with_niche!(TypeAliasId, u64);
new_id_with_niche!(TraitId, u64);
new_id_with_niche!(ImplId, u64);

pub type FuncRef = Idx<FuncId, Func>;
pub type StructRef = Idx<StructId, Struct>;
pub type EnumRef = Idx<EnumId, Enum>;
pub type GlobalRef = Idx<GlobalId, Global>;
pub type TypeAliasRef = Idx<TypeAliasId, TypeAlias>;
pub type TraitRef = Idx<TraitId, Trait>;
pub type ImplRef = Idx<ImplId, Impl>;

#[derive(Clone, Debug)]
pub struct Asg<'a> {
    pub source_files: &'a SourceFiles,
    pub entry_point: Option<FuncRef>,
    pub funcs: Arena<FuncId, Func>,
    pub structs: Arena<StructId, Struct>,
    pub globals: Arena<GlobalId, Global>,
    pub enums: Arena<EnumId, Enum>,
    pub type_aliases: Arena<TypeAliasId, TypeAlias>,
    pub traits: Arena<TraitId, Trait>,
    pub impls: Arena<ImplId, Impl>,
    pub workspace: &'a AstWorkspace<'a>,
}

impl<'a> Asg<'a> {
    pub fn new(workspace: &'a AstWorkspace) -> Self {
        Self {
            source_files: workspace.source_files,
            entry_point: None,
            funcs: Arena::new(),
            structs: Arena::new(),
            globals: Arena::new(),
            enums: Arena::new(),
            type_aliases: Arena::new(),
            traits: Arena::new(),
            impls: Arena::new(),
            workspace,
        }
    }
}
